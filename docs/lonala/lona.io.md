# lona.io

Device I/O intrinsics for driver development.

---

## MMIO Operations

Memory-mapped I/O for device register access.

### `mmio-map`

Map physical device memory.

```clojure
(mmio-map paddr size cache-attr)  ; → vaddr
```

Cache attributes: `:device`, `:uncached`, `:write-combine`, `:cached`

### `mmio-unmap`

Unmap region.

```clojure
(mmio-unmap vaddr)  ; → :ok
```

### Read Operations

```clojure
(mmio-read8 vaddr)     ; → u8
(mmio-read16 vaddr)    ; → u16
(mmio-read32 vaddr)    ; → u32
(mmio-read64 vaddr)    ; → u64
```

### Write Operations

```clojure
(mmio-write8! vaddr val)     ; → :ok
(mmio-write16! vaddr val)    ; → :ok
(mmio-write32! vaddr val)    ; → :ok
(mmio-write64! vaddr val)    ; → :ok
```

### Offset Helpers

```clojure
(mmio-read32-off base offset)       ; → u32
(mmio-write32-off! base offset val) ; → :ok
```

---

## Memory Barriers

### `memory-barrier!`

Full memory barrier.

```clojure
(memory-barrier!)
```

### `read-barrier!`

Load-load barrier.

```clojure
(read-barrier!)
```

### `write-barrier!`

Store-store barrier.

```clojure
(write-barrier!)
```

### `device-barrier!`

Device synchronization barrier.

```clojure
(device-barrier!)
```

### `instruction-barrier!`

Instruction synchronization barrier.

```clojure
(instruction-barrier!)
```

---

## Port I/O (x86)

Legacy I/O port access.

### `port-request`

Request port range capability.

```clojure
(port-request base size)  ; → port-cap
```

### Read Operations

```clojure
(port-in8 cap offset)     ; → u8
(port-in16 cap offset)    ; → u16
(port-in32 cap offset)    ; → u32
```

### Write Operations

```clojure
(port-out8! cap offset val)     ; → :ok
(port-out16! cap offset val)    ; → :ok
(port-out32! cap offset val)    ; → :ok
```

---

## DMA Operations

### `dma-alloc`

Allocate DMA buffer.

```clojure
(dma-alloc size attrs)  ; → dma-buffer
```

Attrs: `:dma/coherent`, `:dma/uncached`, `:dma/write-combine`, `:dma/cached`

### `dma-free!`

Free DMA buffer.

```clojure
(dma-free! buf)  ; → :ok
```

### Buffer Access

```clojure
(dma-vaddr buf)   ; → vaddr
(dma-paddr buf)   ; → paddr
(dma-size buf)    ; → u64
```

### Cache Synchronization

```clojure
(dma-sync-for-device! buf offset size)  ; Before device reads
(dma-sync-for-cpu! buf offset size)     ; After device writes
```

---

## Cache Operations

### `cache-clean!`

Write dirty lines to RAM.

```clojure
(cache-clean! vaddr size)  ; → :ok
```

### `cache-invalidate!`

Discard cache lines.

```clojure
(cache-invalidate! vaddr size)  ; → :ok
```

### `cache-clean-invalidate!`

Clean then invalidate.

```clojure
(cache-clean-invalidate! vaddr size)  ; → :ok
```

### `cache-unify-instruction!`

Sync instruction cache with data cache.

```clojure
(cache-unify-instruction! vaddr size)  ; → :ok
```

---

## IRQ Operations

### `irq-register!`

Register interrupt handler.

```clojure
(irq-register! irq-num notification)  ; → irq-handler
```

### `irq-unregister!`

Remove handler.

```clojure
(irq-unregister! handler)  ; → :ok
```

### `irq-ack!`

Acknowledge interrupt.

```clojure
(irq-ack! handler)  ; → :ok
```

### `irq-wait`

Wait for interrupt. Equivalent to `lona.kernel/wait!`.

```clojure
(irq-wait notification)  ; → badge
```

### `irq-poll`

Poll for interrupt. Equivalent to `lona.kernel/poll!`.

```clojure
(irq-poll notification)  ; → badge or nil
```

### MSI/MSI-X

Register MSI/MSI-X interrupt vectors for PCI devices.

```clojure
(msi-register! pci-dev vector notification)    ; → irq-handler
(msix-register! pci-dev vector notification)   ; → irq-handler
```

**Note:** `pci-dev` is an opaque handle representing a PCI device. The full PCI
discovery and configuration API (enumeration, BAR access, etc.) will be specified
when implementing device drivers.

---

## Physical Memory

### `vaddr->paddr`

Virtual to physical translation.

```clojure
(vaddr->paddr vaddr)  ; → paddr
```

### `paddr->vaddr`

Physical to virtual (if mapped).

```clojure
(paddr->vaddr paddr)  ; → vaddr or nil
```

### `phys-alloc`

Allocate contiguous physical memory.

```clojure
(phys-alloc size alignment)  ; → %{:vaddr :paddr :size :frames}
```

### `phys-free!`

Release physical memory.

```clojure
(phys-free! frames)  ; → :ok
```

---

## Ring Buffers

Lock-free ring buffers for driver communication.

**Concurrency model:** Ring buffers are **SPSC** (Single Producer, Single Consumer).
One realm produces, one realm consumes. `ring-share` grants either `:produce` or
`:consume` access, not both.

### `ring-create`

Create ring buffer.

```clojure
(ring-create num-entries entry-size)  ; → ring
```

### `ring-share`

Share with realm.

```clojure
(ring-share ring realm-id access)  ; → :ok
```

### `ring-destroy!`

Destroy ring.

```clojure
(ring-destroy! ring)  ; → :ok
```

### Producer Operations

```clojure
(ring-produce! ring entry)          ; → :ok or :full
(ring-produce-batch! ring entries)  ; → count
```

### Consumer Operations

```clojure
(ring-consume ring)            ; → entry or nil
(ring-consume-batch ring max)  ; → [entries]
```

### Statistics

```clojure
(ring-available ring)    ; → count
(ring-free-space ring)   ; → count
(ring-full? ring)        ; → boolean
(ring-empty? ring)       ; → boolean
```

---

## Device Tree

Flattened Device Tree access. FDT access is capability-controlled via `fdt-cap?`.
Typically only the root realm and explicitly authorized driver realms hold this
capability.

### `fdt-find-node`

Find node by path.

```clojure
(fdt-find-node path)  ; → node or nil
```

### `fdt-get-property`

Get property value.

```clojure
(fdt-get-property node name)  ; → value or nil
```

### `fdt-get-reg`

Get register address and size.

```clojure
(fdt-get-reg node)  ; → %{:addr paddr :size u64}
```

### `fdt-get-interrupts`

Get interrupt numbers.

```clojure
(fdt-get-interrupts node)  ; → [irq-nums]
```

### `fdt-compatible?`

Check device compatibility.

```clojure
(fdt-compatible? node compat-string)  ; → boolean
```

---

## Appendix: Expected Derived Functions

All functions in this namespace are intrinsics. No derived functions expected.
