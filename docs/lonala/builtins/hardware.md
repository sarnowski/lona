# Hardware Access
> **Status**: *(Planned)*

Low-level hardware access primitives for device drivers.

## MMIO (Memory-Mapped I/O)

Direct hardware register access. These operate on physical memory addresses.

| Function | Syntax | Description |
|----------|--------|-------------|
| `peek-u8` | `(peek-u8 addr)` | Read unsigned 8-bit value |
| `peek-u16` | `(peek-u16 addr)` | Read unsigned 16-bit value |
| `peek-u32` | `(peek-u32 addr)` | Read unsigned 32-bit value |
| `peek-u64` | `(peek-u64 addr)` | Read unsigned 64-bit value |
| `poke-u8` | `(poke-u8 addr val)` | Write unsigned 8-bit value |
| `poke-u16` | `(poke-u16 addr val)` | Write unsigned 16-bit value |
| `poke-u32` | `(poke-u32 addr val)` | Write unsigned 32-bit value |
| `poke-u64` | `(poke-u64 addr val)` | Write unsigned 64-bit value |

### Example: UART Driver

```clojure
(def uart-base 0x09000000)
(poke-u8 uart-base 0x41)      ; Write 'A' to UART data register
(peek-u8 uart-base)           ; Read from UART data register
```

## DMA (Direct Memory Access)

Primitives for zero-copy hardware I/O with physically contiguous memory.

| Function | Syntax | Description |
|----------|--------|-------------|
| `dma-alloc` | `(dma-alloc size)` | Allocate DMA-capable buffer |
| `phys-addr` | `(phys-addr binary)` | Get physical address of buffer |
| `memory-barrier` | `(memory-barrier)` | Ensure memory ordering |

### Example: Network Card Buffer

```clojure
;; Allocate DMA buffer for network card
(def dma-buf (dma-alloc 4096))
;; Returns {:virt <addr> :phys <addr> :buffer <binary>}

;; Get physical address for device descriptor
(phys-addr (:buffer dma-buf))

;; Ensure writes are visible to device
(memory-barrier)
```

## IRQ (Interrupt Handling)

Interrupt handling for device drivers.

| Function | Syntax | Description |
|----------|--------|-------------|
| `irq-wait` | `(irq-wait irq-cap)` | Block until interrupt fires |

### Example: Driver Main Loop

```clojure
(loop []
  (irq-wait uart-irq-cap)
  (handle-uart-interrupt)
  (recur))
```

---

