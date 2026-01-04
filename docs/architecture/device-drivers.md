# Device Drivers

This document covers the device driver architecture in Lona: how drivers are isolated in realms, zero-copy I/O patterns, and hardware interaction.

## Driver Realm Architecture

Each device driver runs in its own isolated realm:

```
DRIVER ISOLATION
════════════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────────┐
│                        DRIVER REALM                                 │
│                                                                     │
│  VSpace:                                                            │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  Lona VM code (shared)                                      │    │
│  │  Driver bytecode (inherited or local)                       │    │
│  │  Process heaps                                              │    │
│  │  MMIO region ← Device registers mapped here                 │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                     │
│  CSpace:                                                            │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  Device frame caps (MMIO)                                   │    │
│  │  IRQ handler caps                                           │    │
│  │  Notification caps (for interrupt delivery)                 │    │
│  │  Endpoint cap (for IPC with Memory Manager)                 │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

Benefits:
- Driver crash doesn't affect other realms
- Driver can't access other devices' memory
- CPU budget limits driver's resource usage
- Capabilities strictly control what driver can access
```

### Driver Capabilities

Drivers receive specific capabilities from the Lona Memory Manager:

```
DRIVER CAPABILITIES
════════════════════════════════════════════════════════════════════════

Device Frame Capabilities:
┌─────────────────────────────────────────────────────────────────────┐
│  device_frames: Vec<FrameCap>                                       │
│                                                                     │
│  - Physical device registers (MMIO)                                 │
│  - DMA buffer regions                                               │
│  - Mapped with appropriate cacheability (uncached, write-combine)   │
└─────────────────────────────────────────────────────────────────────┘

IRQ Handler Capabilities:
┌─────────────────────────────────────────────────────────────────────┐
│  irq_handlers: Vec<IRQHandlerCap>                                   │
│                                                                     │
│  - seL4_IRQHandler capability per interrupt line                    │
│  - Bound to Notification for async delivery                         │
└─────────────────────────────────────────────────────────────────────┘

Notification Capabilities:
┌─────────────────────────────────────────────────────────────────────┐
│  notifications: Vec<NotificationCap>                                │
│                                                                     │
│  - Async signal delivery (interrupts, events)                       │
│  - Driver waits on notification, hardware signals                   │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Network Stack Architecture

A typical network stack separates hardware access from protocol processing:

```
NETWORK STACK DECOMPOSITION
════════════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────────┐
│  APPLICATION REALM                                                  │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  HTTP server, database client, etc.                         │    │
│  │  Uses high-level socket API                                 │    │
│  └─────────────────────────────────────────────────────────────┘    │
└───────────────────────────────┬─────────────────────────────────────┘
                                │ IPC (socket operations)
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  IP STACK REALM                                                     │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  TCP/IP implementation                                      │    │
│  │  Connection state, retransmission, congestion control       │    │
│  │  No hardware access - pure protocol logic                   │    │
│  └─────────────────────────────────────────────────────────────┘    │
└───────────────────────────────┬─────────────────────────────────────┘
                                │ Shared ring buffers (zero-copy)
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  NIC DRIVER REALM                                                   │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  Hardware-specific driver code                              │    │
│  │  MMIO access to NIC registers                               │    │
│  │  DMA descriptor management                                  │    │
│  │  Interrupt handling                                         │    │
│  └─────────────────────────────────────────────────────────────┘    │
└───────────────────────────────┬─────────────────────────────────────┘
                                │ DMA
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  HARDWARE (NIC)                                                     │
└─────────────────────────────────────────────────────────────────────┘

Each layer in its own realm:
- NIC driver crash → IP stack continues with different driver
- IP stack bug → applications unaffected (restart stack)
- Application crash → other apps fine
```

---

## Zero-Copy I/O Patterns

### Shared Ring Buffers

Realms share data through memory-mapped ring buffers:

```
SHARED RING BUFFER
════════════════════════════════════════════════════════════════════════

Producer (Driver)              Consumer (IP Stack)
┌─────────────────┐            ┌─────────────────┐
│                 │            │                 │
│  RX ring: RW    │◀──────────▶│  RX ring: RO    │
│  TX ring: RO    │◀──────────▶│  TX ring: RW    │
│                 │            │                 │
└─────────────────┘            └─────────────────┘
        │                              │
        └──────────────┬───────────────┘
                       │
                       ▼
            ┌─────────────────────┐
            │  SHARED MEMORY      │
            │  (same physical     │
            │   frames, mapped    │
            │   into both realms) │
            └─────────────────────┘

Ring structure:
┌────────────────────────────────────────────────────────────────────┐
│  head: AtomicU32 (producer writes)                                 │
│  tail: AtomicU32 (consumer writes)                                 │
│  entries: [RingEntry; N]                                           │
│     - buffer_ptr: physical address                                 │
│     - length: u32                                                  │
│     - flags: u32                                                   │
└────────────────────────────────────────────────────────────────────┘

Producer:
  1. Get entry at head
  2. Fill buffer, set length
  3. Increment head (atomic)
  4. Signal consumer (notification)

Consumer:
  1. Check head != tail
  2. Process entry at tail
  3. Increment tail (atomic)
  4. Entry now available for reuse
```

### DMA Buffer Management

DMA requires careful memory management:

```
DMA BUFFER ALLOCATION
════════════════════════════════════════════════════════════════════════

DMA buffers must be:
1. Physically contiguous (for hardware)
2. At known physical addresses (for DMA descriptors)
3. Properly aligned (hardware requirement)
4. Accessible by both driver and protocol stack

Allocation flow:
┌─────────────────────────────────────────────────────────────────────┐
│  LONA MEMORY MANAGER                                                │
│                                                                     │
│  Driver requests DMA region:                                        │
│    allocate_dma(size: 1MB, align: 4KB)                              │
│                                                                     │
│  Memory Manager:                                                    │
│    1. Find contiguous Untyped of sufficient size                    │
│    2. Retype to frames                                              │
│    3. Record physical addresses                                     │
│    4. Map into driver's VSpace (uncached)                           │
│    5. Return virtual address + physical address                     │
│                                                                     │
│  For sharing with protocol stack:                                   │
│    6. Map same frames into IP stack's VSpace (RO or RW as needed)   │
└─────────────────────────────────────────────────────────────────────┘
```

### DMA Isolation (IOMMU)

DMA buffer allocation (above) ensures buffers are physically contiguous and properly aligned. But allocation alone doesn't prevent a device from accessing *other* memory. **IOMMU provides the isolation.**

```
WITHOUT IOMMU (Dangerous)
════════════════════════════════════════════════════════════════════════

Device can DMA to ANY physical address:

┌─────────────────┐     DMA to 0x1000_0000     ┌─────────────────────┐
│  NIC Device     │ ──────────────────────────▶│  Allocated Buffer   │ OK
└─────────────────┘                            └─────────────────────┘
        │
        │            DMA to 0x2000_0000     ┌─────────────────────┐
        └─────────────────────────────────▶│  Memory Manager     │ ATTACK!
                                           └─────────────────────┘


WITH IOMMU (Secure)
════════════════════════════════════════════════════════════════════════

IOMMU restricts device to permitted regions only:

┌─────────────────┐                        ┌─────────────────────┐
│  NIC Device     │ ───▶ IOMMU ───────────▶│  Allocated Buffer   │ OK
└─────────────────┘         │              └─────────────────────┘
                            │
                            │  DMA to 0x2000_0000
                            └──────────────────────▶ BLOCKED by IOMMU
```

#### IOMMU Configuration

When Lona boots:

1. **Detection**: Memory Manager probes for IOMMU (VT-d on x86_64, SMMU on aarch64)
2. **If present**:
   - IOMMU is initialized and enabled
   - Each device gets an I/O page table restricting it to allocated DMA regions
   - Driver realms are isolated (can be untrusted)
   - Log: `IOMMU enabled, DMA isolation active`
3. **If absent**:
   - DMA isolation is unavailable
   - Driver realms are trusted (part of TCB)
   - **Warning**: `WARNING: No IOMMU detected. Driver realms are TRUSTED. DMA isolation disabled.`

#### Security Implications

| IOMMU Status | Driver Trust | DMA Protection | Recommendation |
|--------------|--------------|----------------|----------------|
| **Present** | Untrusted | Full | Run any driver code |
| **Absent** | Trusted (TCB) | None | Only run audited drivers |

**Without IOMMU, a malicious or buggy driver can:**

- Read any physical memory (steal secrets from other realms)
- Write any physical memory (corrupt Memory Manager, inject code)
- Completely bypass realm isolation

This is why IOMMU is listed as a hardware requirement for full security. See [Hardware Requirements](index.md#hardware-requirements) and [Supported Hardware](../supported-hardware.md).

#### Platform Support

| Platform | IOMMU | Notes |
|----------|-------|-------|
| QEMU x86_64 (q35) | Intel VT-d | `-device intel-iommu,intremap=on` |
| QEMU aarch64 (virt) | virtio-iommu | `-device virtio-iommu-pci` |
| Servers with VT-d/SMMU | Yes | Most modern server hardware |
| Raspberry Pi 4 | None | No IOMMU hardware |
| Raspberry Pi 5 | Non-standard | Custom Broadcom IOMMUs, not SMMU-compatible |
| Cloud VMs | Varies | Often not exposed to guests |

### Zero-Copy Receive Path

```
ZERO-COPY RX PATH
════════════════════════════════════════════════════════════════════════

1. SETUP (once)
   ┌─────────────────────────────────────────────────────────────────┐
   │  Driver allocates RX buffer pool (e.g., 256 × 2KB buffers)      │
   │  Programs NIC with buffer physical addresses                    │
   │  Shares buffer region with IP stack (RO mapping)                │
   └─────────────────────────────────────────────────────────────────┘

2. PACKET ARRIVES
   ┌─────────────────────────────────────────────────────────────────┐
   │  NIC DMAs packet into next RX buffer                            │
   │  NIC writes descriptor (buffer index, length, status)           │
   │  NIC raises interrupt                                           │
   └─────────────────────────────────────────────────────────────────┘

3. DRIVER HANDLES
   ┌─────────────────────────────────────────────────────────────────┐
   │  Driver receives interrupt notification                         │
   │  Reads completed descriptors                                    │
   │  Posts buffer reference to RX ring (index + length)             │
   │  Signals IP stack                                               │
   └─────────────────────────────────────────────────────────────────┘

4. IP STACK PROCESSES
   ┌─────────────────────────────────────────────────────────────────┐
   │  Reads from RX ring                                             │
   │  Accesses packet data directly (same physical memory!)          │
   │  Processes headers, delivers payload to application             │
   │  Returns buffer index to driver for reuse                       │
   └─────────────────────────────────────────────────────────────────┘

NO COPIES between driver and IP stack!
```

---

## Cache Coherency

DMA bypasses the CPU cache, requiring careful memory management:

```
CACHE COHERENCY
════════════════════════════════════════════════════════════════════════

Problem:
  CPU cache may hold stale data after DMA write
  DMA may read stale data if CPU hasn't flushed

Solutions:

1. UNCACHED MAPPING (simplest)
   ┌─────────────────────────────────────────────────────────────┐
   │  Map DMA buffers as uncached                                │
   │  Every access goes to RAM                                   │
   │  No coherency issues, but slower                            │
   │  Good for: RX buffers (hardware writes frequently)          │
   └─────────────────────────────────────────────────────────────┘

2. WRITE-COMBINE (TX optimization)
   ┌─────────────────────────────────────────────────────────────┐
   │  Writes buffered and combined before going to RAM           │
   │  Good for: TX buffers (CPU writes, hardware reads)          │
   │  Improves write throughput                                  │
   └─────────────────────────────────────────────────────────────┘

3. EXPLICIT CACHE OPERATIONS
   ┌─────────────────────────────────────────────────────────────┐
   │  Before DMA read: cache_flush(buffer)                       │
   │  After DMA write: cache_invalidate(buffer)                  │
   │  More complex, but allows cached access between DMAs        │
   └─────────────────────────────────────────────────────────────┘

Mapping examples:
  RX buffers: uncached (hardware writes frequently)
  TX buffers: write-combine (CPU writes, hardware reads)
  Descriptors: uncached (both read/write frequently)
```

---

## Interrupt Handling

Interrupts are delivered via seL4 Notifications:

```
INTERRUPT FLOW
════════════════════════════════════════════════════════════════════════

1. SETUP
   ┌─────────────────────────────────────────────────────────────────┐
   │  Memory Manager:                                                │
   │    - Creates IRQHandler cap for device's interrupt line         │
   │    - Creates Notification object for driver                     │
   │    - Binds IRQHandler to Notification                           │
   │    - Gives Notification cap to driver realm                     │
   └─────────────────────────────────────────────────────────────────┘

2. INTERRUPT OCCURS
   ┌─────────────────────────────────────────────────────────────────┐
   │  Hardware raises interrupt                                      │
   │  seL4 kernel catches it                                         │
   │  Kernel signals bound Notification                              │
   │  Interrupt masked (won't fire again until acknowledged)         │
   └─────────────────────────────────────────────────────────────────┘

3. DRIVER HANDLES
   ┌─────────────────────────────────────────────────────────────────┐
   │  Driver was waiting: seL4_Wait(notification_cap)                │
   │  Returns with signal bits set                                   │
   │  Driver reads device registers, handles event                   │
   │  Driver acknowledges IRQ: seL4_IRQHandler_Ack(irq_cap)          │
   │  Interrupt unmasked, can fire again                             │
   └─────────────────────────────────────────────────────────────────┘

Driver loop:
  loop {
      bits = seL4_Wait(notification_cap)

      if bits & IRQ_BIT:
          handle_interrupt()
          seL4_IRQHandler_Ack(irq_cap)

      if bits & SHUTDOWN_BIT:
          break
  }
```

---

## MMIO Access

Device registers are accessed through memory-mapped I/O:

```
MMIO ACCESS PATTERNS
════════════════════════════════════════════════════════════════════════

UART Example:
┌─────────────────────────────────────────────────────────────────────┐
│  Physical address: 0x0900_0000 (hardware-defined)                   │
│  Mapped to driver VSpace at: 0x00F0_0000_0000 (arbitrary VA)        │
│  Mapping: uncached, RW                                              │
│                                                                     │
│  struct UartRegs {                                                  │
│      data: Volatile<u32>,     // 0x00: TX/RX data                   │
│      status: Volatile<u32>,   // 0x04: Status flags                 │
│      control: Volatile<u32>,  // 0x08: Control register             │
│  }                                                                  │
│                                                                     │
│  fn write_byte(uart: &UartRegs, byte: u8) {                         │
│      while uart.status.read() & TX_FULL != 0 { }                    │
│      uart.data.write(byte as u32);                                  │
│  }                                                                  │
└─────────────────────────────────────────────────────────────────────┘

Key requirements:
- Volatile access (compiler can't optimize away)
- Proper memory barriers where needed
- Uncached mapping (no stale reads)
```

---

## Driver Lifecycle

```
DRIVER LIFECYCLE
════════════════════════════════════════════════════════════════════════

1. CREATION
   Memory Manager creates driver realm with:
   - Device frame caps (MMIO regions)
   - IRQ handler caps
   - Notification caps
   - Limited CPU/memory budget

2. INITIALIZATION
   Driver:
   - Maps MMIO regions
   - Sets up DMA buffers
   - Configures device
   - Registers interrupt handler
   - Signals ready to parent

3. OPERATION
   Driver:
   - Handles interrupts
   - Processes TX/RX requests
   - Communicates with protocol stacks

4. CRASH/RESTART
   On driver crash:
   - Memory Manager notified (fault handler)
   - Device reset (if possible)
   - New driver realm created
   - State may be lost (protocol stacks handle reconnection)

5. SHUTDOWN
   Orderly shutdown:
   - Drain pending operations
   - Disable device interrupts
   - Unmap MMIO
   - Realm terminated
```

---

## Summary

| Aspect | Description |
|--------|-------------|
| **Isolation** | Each driver in own realm, crash-isolated |
| **Capabilities** | Strict access control via seL4 caps |
| **Zero-copy** | Shared ring buffers between realms |
| **DMA** | Contiguous buffers, known physical addresses |
| **Caching** | Uncached for RX, write-combine for TX |
| **Interrupts** | seL4 Notifications, async delivery |
| **MMIO** | Volatile access, uncached mapping |

This architecture provides strong isolation while enabling efficient zero-copy data paths between drivers and protocol stacks.
