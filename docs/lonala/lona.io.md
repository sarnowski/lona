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

```clojure
;; @todo
(def base (mmio-map (paddr 0x1000u64) 0x1000 :device))
(vaddr? base)  ; => true
```

### `mmio-unmap`

Unmap region.

```clojure
(mmio-unmap vaddr)  ; → :ok
```

```clojure
;; @todo
(def v (mmio-map (paddr 0x2000u64) 0x1000 :device))
(mmio-unmap v)  ; => :ok
```

### Read Operations

```clojure
(mmio-read8 vaddr)     ; → u8
(mmio-read16 vaddr)    ; → u16
(mmio-read32 vaddr)    ; → u32
(mmio-read64 vaddr)    ; → u64
```

```clojure
;; @todo
;; MMIO reads return fixed-width integers
(def base (mmio-map (paddr 0x3000u64) 0x1000 :device))
(integer? (mmio-read8 base))   ; => true
(integer? (mmio-read16 base))  ; => true
(integer? (mmio-read32 base))  ; => true
(integer? (mmio-read64 base))  ; => true
```

### Write Operations

```clojure
(mmio-write8! vaddr val)     ; → :ok
(mmio-write16! vaddr val)    ; → :ok
(mmio-write32! vaddr val)    ; → :ok
(mmio-write64! vaddr val)    ; → :ok
```

```clojure
;; @todo
(def base (mmio-map (paddr 0x4000u64) 0x1000 :device))
(mmio-write8! base 0xFFu8)       ; => :ok
(mmio-write16! base 0x1234u16)   ; => :ok
(mmio-write32! base 0x12345678u32)  ; => :ok
(mmio-write64! base 0x123456789ABCDEFu64)  ; => :ok
```

### Offset Helpers

```clojure
(mmio-read32-off base offset)       ; → u32
(mmio-write32-off! base offset val) ; → :ok
```

```clojure
;; @todo
(def base (mmio-map (paddr 0x5000u64) 0x1000 :device))
(integer? (mmio-read32-off base 4))      ; => true
(mmio-write32-off! base 4 0x12345678u32) ; => :ok
```

---

## Memory Barriers

### `memory-barrier!`

Full memory barrier.

```clojure
(memory-barrier!)
```

```clojure
;; @todo
(memory-barrier!)  ; => nil
```

### `read-barrier!`

Load-load barrier.

```clojure
(read-barrier!)
```

```clojure
;; @todo
(read-barrier!)  ; => nil
```

### `write-barrier!`

Store-store barrier.

```clojure
(write-barrier!)
```

```clojure
;; @todo
(write-barrier!)  ; => nil
```

### `device-barrier!`

Device synchronization barrier.

```clojure
(device-barrier!)
```

```clojure
;; @todo
(device-barrier!)  ; => nil
```

### `instruction-barrier!`

Instruction synchronization barrier.

```clojure
(instruction-barrier!)
```

```clojure
;; @todo
(instruction-barrier!)  ; => nil
```

---

## Port I/O (x86)

Legacy I/O port access.

### `port-request`

Request port range capability.

```clojure
(port-request base size)  ; → port-cap
```

```clojure
;; @todo @x86_64
(def cap (port-request 0x3F8 8))
(port-cap? cap)  ; => true
```

### Read Operations

```clojure
(port-in8 cap offset)     ; → u8
(port-in16 cap offset)    ; → u16
(port-in32 cap offset)    ; → u32
```

```clojure
;; @todo @x86_64
(def cap (port-request 0x3F8 8))
(integer? (port-in8 cap 0))   ; => true
(integer? (port-in16 cap 0))  ; => true
(integer? (port-in32 cap 0))  ; => true
```

### Write Operations

```clojure
(port-out8! cap offset val)     ; → :ok
(port-out16! cap offset val)    ; → :ok
(port-out32! cap offset val)    ; → :ok
```

```clojure
;; @todo @x86_64
(def cap (port-request 0x3F8 8))
(port-out8! cap 0 65)       ; => :ok
(port-out16! cap 0 1000)    ; => :ok
(port-out32! cap 0 100000)  ; => :ok
```

---

## DMA Operations

### `dma-alloc`

Allocate DMA buffer.

```clojure
(dma-alloc size attrs)  ; → dma-buffer
```

Attrs: `:dma/coherent`, `:dma/uncached`, `:dma/write-combine`, `:dma/cached`

```clojure
;; @todo
(def buf (dma-alloc 4096 :dma/coherent))
(dma-buffer? buf)  ; => true
```

### `dma-free!`

Free DMA buffer.

```clojure
(dma-free! buf)  ; → :ok
```

```clojure
;; @todo
(def buf (dma-alloc 4096 :dma/coherent))
(dma-free! buf)  ; => :ok
```

### Buffer Access

```clojure
(dma-vaddr buf)   ; → vaddr
(dma-paddr buf)   ; → paddr
(dma-size buf)    ; → u64
```

```clojure
;; @todo
(def buf (dma-alloc 4096 :dma/coherent))
(vaddr? (dma-vaddr buf))  ; => true
(paddr? (dma-paddr buf))  ; => true
(dma-size buf)            ; => 4096
```

### Cache Synchronization

```clojure
(dma-sync-for-device! buf offset size)  ; Before device reads
(dma-sync-for-cpu! buf offset size)     ; After device writes
```

```clojure
;; @todo
(def buf (dma-alloc 4096 :dma/coherent))
(dma-sync-for-device! buf 0 64)  ; => :ok
(dma-sync-for-cpu! buf 0 64)     ; => :ok
```

---

## Cache Operations

### `cache-clean!`

Write dirty lines to RAM.

```clojure
(cache-clean! vaddr size)  ; → :ok
```

```clojure
;; @todo
(def buf (dma-alloc 4096 :dma/cached))
(cache-clean! (dma-vaddr buf) 4096)  ; => :ok
```

### `cache-invalidate!`

Discard cache lines.

```clojure
(cache-invalidate! vaddr size)  ; → :ok
```

```clojure
;; @todo
(def buf (dma-alloc 4096 :dma/cached))
(cache-invalidate! (dma-vaddr buf) 4096)  ; => :ok
```

### `cache-clean-invalidate!`

Clean then invalidate.

```clojure
(cache-clean-invalidate! vaddr size)  ; → :ok
```

```clojure
;; @todo
(def buf (dma-alloc 4096 :dma/cached))
(cache-clean-invalidate! (dma-vaddr buf) 4096)  ; => :ok
```

### `cache-unify-instruction!`

Sync instruction cache with data cache.

```clojure
(cache-unify-instruction! vaddr size)  ; → :ok
```

```clojure
;; @todo
(def buf (dma-alloc 4096 :dma/cached))
(cache-unify-instruction! (dma-vaddr buf) 4096)  ; => :ok
```

---

## IRQ Operations

### `irq-register!`

Register interrupt handler.

```clojure
(irq-register! irq-num notification)  ; → irq-handler
```

```clojure
;; @todo
(def n (make-notification))
(def h (irq-register! 33 n))
(irq-handler-cap? h)  ; => true
```

### `irq-unregister!`

Remove handler.

```clojure
(irq-unregister! handler)  ; → :ok
```

```clojure
;; @todo
(def n (make-notification))
(def h (irq-register! 33 n))
(irq-unregister! h)  ; => :ok
```

### `irq-ack!`

Acknowledge interrupt.

```clojure
(irq-ack! handler)  ; → :ok
```

```clojure
;; @todo
(def n (make-notification))
(def h (irq-register! 33 n))
(irq-ack! h)  ; => :ok
```

### `irq-wait`

Wait for interrupt. Equivalent to `lona.kernel/wait!`.

```clojure
(irq-wait notification)  ; → badge
```

```clojure
;; @todo
(def n (make-notification))
(def h (irq-register! 33 n))
;; irq-wait blocks until interrupt fires
;; Signal notification to unblock
(signal! n)
(integer? (irq-wait n))  ; => true
(irq-unregister! h)
```

### `irq-poll`

Poll for interrupt. Equivalent to `lona.kernel/poll!`.

```clojure
(irq-poll notification)  ; → badge or nil
```

```clojure
;; @todo
(def n (make-notification))
(irq-poll n)  ; => nil
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

```clojure
;; @todo
;; MSI registration requires PCI device handle
(def n (make-notification))
;; Would use actual PCI device in real code:
;; (def h (msi-register! pci-dev 0 n))
;; (irq-handler-cap? h)  ; => true
```

---

## Physical Memory

### `vaddr->paddr`

Virtual to physical translation.

```clojure
(vaddr->paddr vaddr)  ; → paddr
```

```clojure
;; @todo
(def buf (dma-alloc 4096 :dma/coherent))
(paddr? (vaddr->paddr (dma-vaddr buf)))  ; => true
```

### `paddr->vaddr`

Physical to virtual (if mapped).

```clojure
(paddr->vaddr paddr)  ; → vaddr or nil
```

```clojure
;; @todo
;; Unmapped address returns nil
(paddr->vaddr (paddr 0xDEADBEEFu64))  ; => nil

;; Mapped address returns vaddr
(def buf (dma-alloc 4096 :dma/coherent))
(vaddr? (paddr->vaddr (dma-paddr buf)))  ; => true
```

### `phys-alloc`

Allocate contiguous physical memory.

```clojure
(phys-alloc size alignment)  ; → %{:vaddr :paddr :size :frames}
```

```clojure
;; @todo
(def r (phys-alloc 4096 4096))
(map? r)                ; => true
(contains? r :vaddr)    ; => true
(contains? r :paddr)    ; => true
(contains? r :size)     ; => true
(contains? r :frames)   ; => true
(vaddr? (:vaddr r))     ; => true
(paddr? (:paddr r))     ; => true
(= (:size r) 4096)      ; => true
```

### `phys-free!`

Release physical memory.

```clojure
(phys-free! frames)  ; → :ok
```

```clojure
;; @todo
(def r (phys-alloc 4096 4096))
(phys-free! (:frames r))  ; => :ok
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

```clojure
;; @todo
(def r (ring-create 64 256))
(ring? r)  ; => true
```

### `ring-share`

Share with realm.

```clojure
(ring-share ring realm-id access)  ; → :ok
```

```clojure
;; @todo
(def r (ring-create 64 256))
(def child (realm-create %{:name 'ring-consumer}))
(ring-share r child :consume)  ; => :ok
(realm-terminate child)
(ring-destroy! r)
```

### `ring-destroy!`

Destroy ring.

```clojure
(ring-destroy! ring)  ; → :ok
```

```clojure
;; @todo
(def r (ring-create 64 256))
(ring-destroy! r)  ; => :ok
```

### Producer Operations

```clojure
(ring-produce! ring entry)          ; → :ok or :full
(ring-produce-batch! ring entries)  ; → count
```

```clojure
;; @todo
(def r (ring-create 4 8))
(ring-produce! r #bytes[1 2 3 4])  ; => :ok
```

### Consumer Operations

```clojure
(ring-consume ring)            ; → entry or nil
(ring-consume-batch ring max)  ; → [entries]
```

```clojure
;; @todo
(def r (ring-create 4 8))
(ring-produce! r #bytes[1 2 3 4])
(binary? (ring-consume r))  ; => true
```

Batch operations:

```clojure
;; @todo
(def r (ring-create 8 8))
(ring-produce-batch! r {#bytes[1] #bytes[2] #bytes[3]})  ; => 3
(ring-available r)  ; => 3

(def entries (ring-consume-batch r 10))
(vector? entries)   ; => true
(count entries)     ; => 3
(ring-destroy! r)
```

### Statistics

```clojure
(ring-available ring)    ; → count
(ring-free-space ring)   ; → count
(ring-full? ring)        ; → boolean
(ring-empty? ring)       ; → boolean
```

```clojure
;; @todo
(def r (ring-create 4 8))
(ring-empty? r)       ; => true
(ring-full? r)        ; => false
(ring-available r)    ; => 0
(ring-free-space r)   ; => 4
```

```clojure
;; @todo
(def r (ring-create 4 8))
(ring-produce! r #bytes[1])
(ring-produce! r #bytes[2])
(ring-empty? r)       ; => false
(ring-available r)    ; => 2
(ring-free-space r)   ; => 2
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

```clojure
;; @todo
(nil? (fdt-find-node "/nonexistent-node"))  ; => true
```

### `fdt-get-property`

Get property value.

```clojure
(fdt-get-property node name)  ; → value or nil
```

```clojure
;; @todo
(def root (fdt-find-node "/"))
(fdt-get-property root "nonexistent")  ; => nil
```

### `fdt-get-reg`

Get register address and size.

```clojure
(fdt-get-reg node)  ; → %{:addr paddr :size u64}
```

```clojure
;; @todo
;; fdt-get-reg returns nil for nodes without reg property
(def root (fdt-find-node "/"))
(or (nil? (fdt-get-reg root))
    (map? (fdt-get-reg root)))  ; => true
```

### `fdt-get-interrupts`

Get interrupt numbers.

```clojure
(fdt-get-interrupts node)  ; → [irq-nums]
```

```clojure
;; @todo
;; fdt-get-interrupts returns vector or nil
(def root (fdt-find-node "/"))
(or (nil? (fdt-get-interrupts root))
    (vector? (fdt-get-interrupts root)))  ; => true
```

### `fdt-compatible?`

Check device compatibility.

```clojure
(fdt-compatible? node compat-string)  ; → boolean
```

```clojure
;; @todo
(def root (fdt-find-node "/"))
(boolean? (fdt-compatible? root "some-compat-string"))  ; => true
```

---

## Appendix: Expected Derived Functions

All functions in this namespace are intrinsics. No derived functions expected.
