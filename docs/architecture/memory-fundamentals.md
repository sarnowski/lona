# Memory Fundamentals

This document covers hardware memory concepts, the MMU, and seL4's memory model.

## Physical Memory and Hardware

At boot, hardware provides a **physical address space** - a flat range of addresses mapping to DRAM chips and memory-mapped devices (MMIO). The address space width is determined by CPU architecture (e.g., 48-bit physical on x86_64).

### Memory Map from Firmware

Firmware (UEFI/BIOS) provides a memory map describing what exists at each physical address:

```
Physical Address Space (example)
┌──────────────────────────────────────────────────────────────┐
│  0x0000_0000 - 0x0009_FFFF  │ Low memory (legacy/reserved)   │
├──────────────────────────────────────────────────────────────┤
│  0x0010_0000 - 0xBFFF_FFFF  │ Usable RAM (~3GB)              │
├──────────────────────────────────────────────────────────────┤
│  0xC000_0000 - 0xCFFF_FFFF  │ Reserved (ACPI, firmware)      │
├──────────────────────────────────────────────────────────────┤
│  0xD000_0000 - 0xEFFF_FFFF  │ PCI MMIO (devices)             │
├──────────────────────────────────────────────────────────────┤
│  0xFEC0_0000 - 0xFEE0_0FFF  │ APIC, HPET (CPU/chipset)       │
├──────────────────────────────────────────────────────────────┤
│  0xFF00_0000 - 0xFFFF_FFFF  │ Firmware ROM                   │
└──────────────────────────────────────────────────────────────┘
```

### Variable Memory Sizes

Different machines have different amounts of RAM. The physical address space is a fixed-width coordinate system, but actual RAM only populates some addresses:

```
4GB Machine:
┌──────────┬────────┐
│   RAM    │ MMIO   │
│  (3GB)   │(hole)  │
└──────────┴────────┘
0GB        3GB      4GB

128GB Machine:
┌──────────┬────────┬─────────────────────────────────────────┐
│   RAM    │ MMIO   │                   RAM                   │
│  (3GB)   │(hole)  │                (~125GB)                 │
└──────────┴────────┴─────────────────────────────────────────┘
0GB        3GB      4GB                                      128GB+
```

The **MMIO hole** (typically in the 3-4GB region on x86) contains fixed addresses for hardware devices. On a 4GB machine, this "steals" ~1GB of usable RAM. Larger machines continue RAM above the hole.

### Accessing Non-Existent Addresses

Accessing a physical address with no RAM or device causes a machine check exception, bus error, or returns garbage (platform-dependent).

---

## The MMU (Memory Management Unit)

The MMU is hardware within the CPU that translates **virtual addresses** (what programs use) to **physical addresses** (actual RAM locations).

```
┌─────────┐     Virtual Addr     ┌─────────┐    Physical Addr    ┌─────────┐
│   CPU   │ ────────────────────▶│   MMU   │────────────────────▶│   RAM   │
└─────────┘                      └────┬────┘                     └─────────┘
                                      │
                                      │ Consults
                                      ▼
                                ┌───────────┐
                                │Page Tables│
                                │ (in RAM)  │
                                └───────────┘
```

### Page Tables

Memory is divided into fixed-size **pages** (typically 4KB). The MMU uses hierarchical page tables stored in RAM to translate addresses.

**x86_64 4-level paging:**

```
Virtual Address (48 bits used):
┌─────────┬─────────┬─────────┬─────────┬──────────────┐
│ PML4    │  PDPT   │   PD    │   PT    │   Offset     │
│ (9 bits)│ (9 bits)│ (9 bits)│ (9 bits)│  (12 bits)   │
└────┬────┴────┬────┴────┬────┴────┬────┴──────┬───────┘
     │         │         │         │           │
     ▼         ▼         ▼         ▼           │
   PML4 ──▶ PDPT ──▶ PD ──▶ PT ──▶ Frame       │
                                   │           │
                                   ▼           ▼
                          Physical Address = Frame Base + Offset
```

Each page table entry contains:
- **Physical frame address** - where the page lives in RAM
- **Present bit** - is this page mapped?
- **Read/Write bit** - writable or read-only?
- **User/Supervisor bit** - accessible from userspace?
- **Execute disable (NX)** - can code execute here?

### TLB (Translation Lookaside Buffer)

Page table walks are slow (4 memory reads for one access). The **TLB** caches recent translations:

```
Virtual 0x7FFF_1234 → TLB hit? → Physical 0x1234_5234
                          │
                     TLB miss → Walk page tables → Cache result
```

When switching address spaces, the TLB must be flushed (or use ASID/PCID tags).

---

## Virtual Address Spaces

Each process gets its own **virtual address space** - the illusion of having all memory to itself:

```
Process A's View              Process B's View
┌──────────────────┐          ┌──────────────────┐
│ 0xFFFF...        │          │ 0xFFFF...        │
│ Kernel (shared)  │          │ Kernel (shared)  │
├──────────────────┤          ├──────────────────┤
│ Stack            │          │ Stack            │
├──────────────────┤          ├──────────────────┤
│ Heap             │          │ Heap             │
├──────────────────┤          ├──────────────────┤
│ Code + Data      │          │ Code + Data      │
├──────────────────┤          ├──────────────────┤
│ 0x0000...        │          │ 0x0000...        │
└──────────────────┘          └──────────────────┘

Same virtual address 0x1000 maps to DIFFERENT physical frames!
```

The kernel maintains separate page tables for each process. On context switch, it loads the new page table root into the CPU's control register (CR3 on x86, TTBR0 on ARM).

### Traditional OS: Implicit Memory Management

Traditional kernels hide memory complexity:

1. **Demand paging**: Physical memory allocated lazily when pages are first accessed
2. **Stack growth**: Kernel automatically maps more pages on stack overflow
3. **Heap growth**: `brk()`/`mmap()` extends virtual mappings, physical allocated on fault
4. **OOM killer**: When physical memory is exhausted, kernel kills processes

---

## seL4's Memory Model

seL4 fundamentally differs from traditional kernels: **the kernel does nothing automatically**. It provides mechanism but no policy.

### Untyped Memory

At boot, seL4 gives the root task capabilities to **Untyped Memory** - raw physical memory regions:

```
seL4 Kernel at Boot:
┌──────────────────────────────────────────────────────────────────┐
│  "Here's all RAM as Untyped capabilities. I just enforce rules." │
│                                                                  │
│  Untyped(0x1000_0000, 2^20)  ← 1MB region                        │
│  Untyped(0x1010_0000, 2^24)  ← 16MB region                       │
│  Untyped(0x2000_0000, 2^30)  ← 1GB region                        │
│  ...                                                             │
└──────────────────────────────────────────────────────────────────┘
```

Untyped memory is **retyped** into kernel objects:

```
Untyped ──retype──▶ VSpace      (address space)
        ──retype──▶ Frame       (physical page)
        ──retype──▶ PageTable   (page table structure)
        ──retype──▶ TCB         (thread control block)
        ──retype──▶ Endpoint    (IPC channel)
        ──retype──▶ CNode       (capability storage)
```

### VSpace Objects

A **VSpace** is seL4's abstraction over an address space. On ARM64:
- **VSpace** - contains PGD (Page Global Directory)
- **PageUpperDirectory** - PUD level
- **PageDirectory** - PD level
- **PageTable** - PT level (points to frames)

### Mapping Memory

To map memory in seL4, you must:

1. Have capability to a Frame (physical memory)
2. Have capability to the VSpace
3. Have/create page table objects at each level
4. Call the map operation with specific permissions

```
Mapping 0x4000_0000 in a VSpace:

1. vspace_cap      ← capability to the VSpace
2. frame_cap       ← capability to a 4KB Frame
3. pud_cap         ← create PageUpperDirectory, map to VSpace
4. pd_cap          ← create PageDirectory, map to PUD
5. pt_cap          ← create PageTable, map to PD
6. frame.map(vspace_cap, 0x4000_0000, rights, attrs)

Result: Virtual 0x4000_0000 now points to the physical frame
```

### Page Faults in seL4

When a thread accesses an unmapped virtual address:

```
Traditional Kernel:
  Page fault → Kernel handles internally → Thread resumes (transparent)

seL4:
  Page fault → Kernel sends IPC to fault handler endpoint →
  Userspace fault handler receives fault info →
  Handler decides what to do (map memory, kill thread, etc.) →
  Handler replies → Thread resumes
```

The fault handler is a userspace component. seL4 just forwards the fault; it never allocates memory itself.

### Key Properties

| Property | seL4 Behavior |
|----------|---------------|
| **No kernel allocator** | Kernel never allocates; it only retypes |
| **Capability protection** | Can't map memory without capability |
| **Explicit structure** | Must create intermediate page tables yourself |
| **Userspace policy** | Fault handling, OOM policy all in userspace |

---

## Memory Allocation Flow

Complete flow for allocating memory in seL4:

```
Step 1: Start with Untyped capability
────────────────────────────────────────────────────────────────────
  untyped_cap = capability to Untyped(phys: 0x8000_0000, size: 2^12)

Step 2: Retype into a Frame
────────────────────────────────────────────────────────────────────
  seL4_Untyped_Retype(
      untyped_cap,
      seL4_ARM_SmallPageObject,  // 4KB frame
      dest_cnode, dest_slot      // where to put new cap
  )
  → frame_cap now exists in dest_slot

Step 3: Ensure page table structure exists
────────────────────────────────────────────────────────────────────
  For each missing level (PUD, PD, PT):
    - Retype more untyped into page table object
    - Map page table to parent level

Step 4: Map the Frame into VSpace
────────────────────────────────────────────────────────────────────
  seL4_ARM_Page_Map(
      frame_cap,
      vspace_cap,
      vaddr: 0x1_0000_0000,     // virtual address
      rights: seL4_ReadWrite,   // permissions
      attrs: seL4_Default       // cacheability
  )

Result: Virtual address 0x1_0000_0000 now accessible
```

---

## Key Invariants

1. **Conservation**: Total physical memory never changes - only ownership transfers
2. **Capability mediation**: Every memory operation requires a capability
3. **No kernel allocation**: Kernel only transforms objects, never creates from nothing
4. **Hierarchical delegation**: Parent gives memory to children, can revoke
