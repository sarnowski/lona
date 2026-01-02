# Lonala I/O Specification

> **Namespace:** `lona.io`

This document specifies the device driver primitives for Lonala, enabling development of hardware drivers for block devices, network cards, graphics cards, UART, and other peripherals.

**Related:** [lonala.md](lonala.md) (core language) | [lonala-process.md](lonala-process.md) (process primitives) | [lonala-kernel.md](lonala-kernel.md) (kernel primitives)

---

## Table of Contents

1. [Overview](#overview)
2. [MMIO Operations](#mmio-operations)
3. [Memory Barriers](#memory-barriers)
4. [Port I/O](#port-io-x86-only)
5. [DMA Operations](#dma-operations)
6. [Cache Operations](#cache-operations)
7. [IRQ Operations](#irq-operations)
8. [Physical Memory](#physical-memory)
9. [Ring Buffers](#ring-buffers)
10. [Device Tree](#device-tree)
11. [API Reference](#api-reference)
12. [Examples](#examples)

---

## Overview

The `lona.io` namespace provides primitives for writing device drivers:

- **MMIO**: Memory-mapped I/O for device register access
- **Port I/O**: Legacy x86 I/O port access
- **DMA**: Direct Memory Access buffer management
- **IRQ**: Interrupt handling
- **Cache**: Cache coherency management
- **Ring Buffers**: Zero-copy data sharing

These primitives are used by driver realms that have been granted appropriate capabilities by the root realm.

---

## MMIO Operations

Memory-Mapped I/O allows accessing device registers through memory addresses.

### `mmio-map`

Map physical device memory into virtual address space.

```clojure
(mmio-map paddr size cache-attr)  ; → vaddr
```

**Parameters:**
- `paddr` — Physical address of device registers
- `size` — Size of region in bytes
- `cache-attr` — Cache attributes (see below)

**Cache Attributes:**
| Attribute | Description | Use Case |
|-----------|-------------|----------|
| `:device` | Device memory (strongly ordered) | MMIO registers |
| `:uncached` | No caching | DMA descriptors |
| `:write-combine` | Write-combining | Frame buffers, TX buffers |
| `:cached` | Normal cached | General memory |

**Example:**
```clojure
(def uart-regs (mmio-map (paddr 0x1000_0000u64) 0x1000u64 :device))
```

---

### `mmio-unmap`

Unmap previously mapped region.

```clojure
(mmio-unmap vaddr)  ; → :ok
```

---

### Register Access

Volatile read/write operations that prevent compiler reordering.

```clojure
;; Read operations
(mmio-read8 vaddr)    ; → u8
(mmio-read16 vaddr)   ; → u16
(mmio-read32 vaddr)   ; → u32
(mmio-read64 vaddr)   ; → u64

;; Write operations
(mmio-write8! vaddr val)    ; → :ok
(mmio-write16! vaddr val)   ; → :ok
(mmio-write32! vaddr val)   ; → :ok
(mmio-write64! vaddr val)   ; → :ok
```

**Example:**
```clojure
;; Read status register
(def status (mmio-read32 (vaddr+ uart-regs 0x14u64)))

;; Write data register
(mmio-write8! (vaddr+ uart-regs 0x00u64) (u8 \H))
```

---

### Offset Helpers

For cleaner register access with base + offset pattern:

```clojure
(mmio-read32-off base offset)    ; Same as (mmio-read32 (vaddr+ base offset))
(mmio-write32-off! base offset val)
```

---

## Memory Barriers

Ensure ordering of memory operations across CPUs and devices.

### `memory-barrier!`

Full memory barrier (DMB on ARM, MFENCE on x86).

```clojure
(memory-barrier!)
```

Ensures all prior loads and stores complete before subsequent ones.

---

### `read-barrier!`

Load-load barrier.

```clojure
(read-barrier!)
```

Ensures all prior loads complete before subsequent loads.

---

### `write-barrier!`

Store-store barrier.

```clojure
(write-barrier!)
```

Ensures all prior stores complete before subsequent stores.

---

### `device-barrier!`

Device synchronization barrier (DSB on ARM).

```clojure
(device-barrier!)
```

Ensures all prior memory accesses, including device memory, are complete.

---

### `instruction-barrier!`

Instruction synchronization barrier (ISB on ARM).

```clojure
(instruction-barrier!)
```

Flushes pipeline, ensures subsequent instructions see prior changes.

---

## Port I/O (x86 only)

Legacy x86 I/O port access for devices that don't use MMIO.

### `port-request`

Request capability to access I/O port range.

```clojure
(port-request base size)  ; → port-cap
```

**Example:**
```clojure
(def com1-ports (port-request 0x3F8u16 8u16))
```

---

### Port Access

```clojure
;; Read from port
(port-in8 port-cap offset)    ; → u8
(port-in16 port-cap offset)   ; → u16
(port-in32 port-cap offset)   ; → u32

;; Write to port
(port-out8! port-cap offset val)    ; → :ok
(port-out16! port-cap offset val)   ; → :ok
(port-out32! port-cap offset val)   ; → :ok
```

**Example:**
```clojure
;; Read from COM1 data register
(def data (port-in8 com1-ports 0u16))

;; Write to COM1 data register
(port-out8! com1-ports 0u16 (u8 \X))
```

---

## DMA Operations

Direct Memory Access buffer management for high-performance I/O.

### `dma-alloc`

Allocate DMA-capable buffer.

```clojure
(dma-alloc size attrs)  ; → dma-buffer
```

**Attributes:**
| Attribute | Description |
|-----------|-------------|
| `:dma/coherent` | Hardware-coherent (no manual sync) |
| `:dma/uncached` | Uncached (for descriptors) |
| `:dma/write-combine` | Write-combining (for TX) |
| `:dma/cached` | Cached (requires manual sync) |

**Returns:** `dma-buffer` handle (see lonala.md System Types)

**Example:**
```clojure
(def tx-buffer (dma-alloc 4096u64 :dma/write-combine))
;; Access with:
(dma-vaddr tx-buffer)   ; → vaddr (for CPU access)
(dma-paddr tx-buffer)   ; → paddr (for device programming)
(dma-size tx-buffer)    ; → u64
```

---

### `dma-free!`

Free DMA buffer.

```clojure
(dma-free! dma-buffer)  ; → :ok
```

---

### Buffer Access

```clojure
(dma-vaddr dma-buffer)   ; → virtual address
(dma-paddr dma-buffer)   ; → physical address
(dma-size dma-buffer)    ; → size in bytes
```

---

### Cache Synchronization

For non-coherent DMA buffers, manual synchronization is required.

### `dma-sync-for-device!`

Flush CPU cache before device reads buffer.

```clojure
(dma-sync-for-device! dma-buffer offset size)  ; → :ok
```

Call **before** device DMA read (CPU → Device transfer).

---

### `dma-sync-for-cpu!`

Invalidate cache after device writes buffer.

```clojure
(dma-sync-for-cpu! dma-buffer offset size)  ; → :ok
```

Call **after** device DMA write (Device → CPU transfer).

---

**DMA Synchronization Pattern:**

```clojure
;; TX (CPU writes, device reads)
(copy-to-buffer! tx-buffer data)
(dma-sync-for-device! tx-buffer 0 (count data))
(start-tx-dma! (dma-paddr tx-buffer) (count data))

;; RX (Device writes, CPU reads)
(wait-for-rx-complete!)
(dma-sync-for-cpu! rx-buffer 0 rx-len)
(def data (read-from-buffer rx-buffer rx-len))
```

---

## Cache Operations

Direct cache control for advanced use cases.

### `cache-clean!`

Write dirty cache lines to RAM.

```clojure
(cache-clean! vaddr size)  ; → :ok
```

---

### `cache-invalidate!`

Discard cache lines (data may be lost).

```clojure
(cache-invalidate! vaddr size)  ; → :ok
```

**Warning:** Only use when you know the data is stale.

---

### `cache-clean-invalidate!`

Clean then invalidate cache lines.

```clojure
(cache-clean-invalidate! vaddr size)  ; → :ok
```

---

### `cache-unify-instruction!`

Synchronize instruction cache with data cache.

```clojure
(cache-unify-instruction! vaddr size)  ; → :ok
```

Required after writing code to memory (JIT compilation).

---

## IRQ Operations

Hardware interrupt handling.

### `irq-register!`

Register handler for hardware interrupt.

```clojure
(irq-register! irq-num notification)  ; → irq-handler
```

**Parameters:**
- `irq-num` — Hardware IRQ number
- `notification` — Notification object to signal on interrupt

**Returns:** IRQ handler capability

**Example:**
```clojure
(def ntfn (make-notification))
(def uart-irq (irq-register! 33 ntfn))
```

---

### `irq-unregister!`

Remove interrupt handler.

```clojure
(irq-unregister! irq-handler)  ; → :ok
```

---

### `irq-ack!`

Acknowledge interrupt and re-enable.

```clojure
(irq-ack! irq-handler)  ; → :ok
```

Must be called after handling interrupt to receive future interrupts.

---

### `irq-wait`

Wait for interrupt (blocking).

```clojure
(irq-wait notification)  ; → badge
```

Blocks until interrupt occurs. Returns notification badge.

---

### `irq-poll`

Check for interrupt (non-blocking).

```clojure
(irq-poll notification)  ; → badge or nil
```

---

### MSI/MSI-X Interrupts

For modern PCI devices using Message Signaled Interrupts.

```clojure
(msi-register! pci-device vector notification)   ; → irq-handler
(msix-register! pci-device vector notification)  ; → irq-handler
```

---

**IRQ Handling Pattern:**

```clojure
(defn irq-handler-loop [ntfn irq device]
  (irq-wait ntfn)
  (let [status (read-device-status device)]
    (handle-interrupt device status)
    (irq-ack! irq))
  (irq-handler-loop ntfn irq device))  ; TCO handles tail recursion
```

---

## Physical Memory

Operations for physical memory management.

### `vaddr->paddr`

Translate virtual address to physical.

```clojure
(vaddr->paddr vaddr)  ; → paddr
```

---

### `paddr->vaddr`

Get virtual address for physical address (if mapped).

```clojure
(paddr->vaddr paddr)  ; → vaddr or nil
```

---

### `phys-alloc`

Allocate contiguous physical memory.

```clojure
(phys-alloc size alignment)
;; → %{:vaddr vaddr :paddr paddr :size u64 :frames [frame-cap ...]}
```

**Example:**
```clojure
;; Allocate 1MB aligned to 4KB
(def mem (phys-alloc (* 1024u64 1024u64) 4096u64))
```

---

### `phys-free!`

Release physical memory.

```clojure
(phys-free! frames)  ; → :ok
```

---

## Ring Buffers

Lock-free ring buffers for high-performance driver communication.

### `ring-create`

Create a ring buffer.

```clojure
(ring-create num-entries entry-size)  ; → ring
```

**Example:**
```clojure
(def rx-ring (ring-create 256u32 16u32))  ; 256 entries, 16 bytes each
```

---

### `ring-share`

Share ring buffer with another realm.

```clojure
(ring-share ring realm-id access)  ; → :ok
```

**Access:** `:read-only` or `:read-write`

---

### `ring-destroy!`

Destroy ring buffer.

```clojure
(ring-destroy! ring)  ; → :ok
```

---

### Producer Operations

```clojure
(ring-produce! ring entry)        ; → :ok or :full
(ring-produce-batch! ring entries) ; → count-added
```

---

### Consumer Operations

```clojure
(ring-consume ring)               ; → entry or nil
(ring-consume-batch ring max)     ; → [entries]
```

---

### Statistics

```clojure
(ring-available ring)    ; → count of available entries
(ring-free-space ring)   ; → count of free slots
(ring-full? ring)        ; → boolean
(ring-empty? ring)       ; → boolean
```

---

## Device Tree

Access Flattened Device Tree (FDT) for device discovery.

### `fdt-find-node`

Find node by path.

```clojure
(fdt-find-node path)  ; → node or nil
```

**Example:**
```clojure
(fdt-find-node "/soc/serial@10000000")
```

---

### `fdt-get-property`

Get property value from node.

```clojure
(fdt-get-property node name)  ; → value or nil
```

---

### `fdt-get-reg`

Get register address and size.

```clojure
(fdt-get-reg node)  ; → %{:addr paddr :size u64}
```

---

### `fdt-get-interrupts`

Get interrupt numbers.

```clojure
(fdt-get-interrupts node)  ; → [irq-nums]
```

---

### `fdt-compatible?`

Check device compatibility.

```clojure
(fdt-compatible? node compat-string)  ; → boolean
```

**Example:**
```clojure
(fdt-compatible? uart-node "ns16550a")
```

---

## API Reference

### MMIO

```clojure
(mmio-map paddr size attr)          ; → vaddr
(mmio-unmap vaddr)                  ; → :ok
(mmio-read8 vaddr)                  ; → u8
(mmio-read16 vaddr)                 ; → u16
(mmio-read32 vaddr)                 ; → u32
(mmio-read64 vaddr)                 ; → u64
(mmio-write8! vaddr val)            ; → :ok
(mmio-write16! vaddr val)           ; → :ok
(mmio-write32! vaddr val)           ; → :ok
(mmio-write64! vaddr val)           ; → :ok
```

### Memory Barriers

```clojure
(memory-barrier!)                   ; Full barrier
(read-barrier!)                     ; Load-load
(write-barrier!)                    ; Store-store
(device-barrier!)                   ; Device sync
(instruction-barrier!)              ; Pipeline flush
```

### Port I/O (x86)

```clojure
(port-request base size)            ; → port-cap
(port-in8 cap offset)               ; → u8
(port-in16 cap offset)              ; → u16
(port-in32 cap offset)              ; → u32
(port-out8! cap offset val)         ; → :ok
(port-out16! cap offset val)        ; → :ok
(port-out32! cap offset val)        ; → :ok
```

### DMA

```clojure
(dma-alloc size attrs)              ; → dma-buffer
(dma-free! buf)                     ; → :ok
(dma-vaddr buf)                     ; → vaddr
(dma-paddr buf)                     ; → paddr
(dma-size buf)                      ; → size
(dma-sync-for-device! buf off size) ; → :ok
(dma-sync-for-cpu! buf off size)    ; → :ok
```

### Cache

```clojure
(cache-clean! vaddr size)           ; → :ok
(cache-invalidate! vaddr size)      ; → :ok
(cache-clean-invalidate! vaddr size); → :ok
(cache-unify-instruction! vaddr size); → :ok
```

### IRQ

```clojure
(irq-register! irq-num ntfn)        ; → handler
(irq-unregister! handler)           ; → :ok
(irq-ack! handler)                  ; → :ok
(irq-wait ntfn)                     ; → badge
(irq-poll ntfn)                     ; → badge or nil
(msi-register! pci-dev vec ntfn)    ; → handler
(msix-register! pci-dev vec ntfn)   ; → handler
```

### Physical Memory

```clojure
(vaddr->paddr vaddr)                ; → paddr
(paddr->vaddr paddr)                ; → vaddr or nil
(phys-alloc size align)             ; → %{:vaddr :paddr :size :frames}
(phys-free! frames)                 ; → :ok
```

### Ring Buffers

```clojure
(ring-create entries entry-size)    ; → ring
(ring-share ring realm access)      ; → :ok
(ring-destroy! ring)                ; → :ok
(ring-produce! ring entry)          ; → :ok or :full
(ring-produce-batch! ring entries)  ; → count
(ring-consume ring)                 ; → entry or nil
(ring-consume-batch ring max)       ; → [entries]
(ring-available ring)               ; → count
(ring-free-space ring)              ; → count
```

### Device Tree

```clojure
(fdt-find-node path)                ; → node or nil
(fdt-get-property node name)        ; → value or nil
(fdt-get-reg node)                  ; → %{:addr :size}
(fdt-get-interrupts node)           ; → [irq-nums]
(fdt-compatible? node compat)       ; → boolean
```

---

## Examples

### UART Driver

```clojure
(ns drivers.uart
  (:require [lona.io :as io]
            [lona.process :as proc]))

;; NS16550 UART register offsets
(def +RBR+ 0x00u64)  ; Receive Buffer
(def +THR+ 0x00u64)  ; Transmit Holding
(def +IER+ 0x04u64)  ; Interrupt Enable
(def +FCR+ 0x08u64)  ; FIFO Control
(def +LCR+ 0x0Cu64)  ; Line Control
(def +LSR+ 0x14u64)  ; Line Status
(def +LSR-DR+   0x01u8)  ; Data Ready
(def +LSR-THRE+ 0x20u8)  ; TX Holding Empty

(defn uart-init [paddr irq-num]
  (let [regs (io/mmio-map paddr 0x100u64 :device)
        ntfn (proc/make-notification)
        irq  (io/irq-register! irq-num ntfn)]
    ;; Configure UART: 8N1, FIFO enabled
    (io/mmio-write8! (vaddr+ regs +LCR+) 0x03u8)
    (io/mmio-write8! (vaddr+ regs +FCR+) 0x07u8)
    (io/mmio-write8! (vaddr+ regs +IER+) 0x01u8)  ; Enable RX interrupt
    %{:regs regs :irq irq :ntfn ntfn}))

(defn uart-tx-ready? [regs]
  (not (zero? (bit-and (io/mmio-read8 (vaddr+ regs +LSR+)) +LSR-THRE+))))

(defn uart-rx-ready? [regs]
  (not (zero? (bit-and (io/mmio-read8 (vaddr+ regs +LSR+)) +LSR-DR+))))

(defn uart-putc [%{:keys [regs]} ch]
  (letfn [(wait-tx []
            (if (uart-tx-ready? regs)
              :ready
              (wait-tx)))]
    (wait-tx)
    (io/mmio-write8! (vaddr+ regs +THR+) (u8 ch))))

(defn uart-getc [%{:keys [regs ntfn irq]}]
  (letfn [(wait-rx []
            (if (uart-rx-ready? regs)
              (io/mmio-read8 (vaddr+ regs +RBR+))
              (do
                (io/irq-wait ntfn)
                (io/irq-ack! irq)
                (wait-rx))))]
    (wait-rx)))

(defn uart-write [uart data]
  (doseq [ch data]
    (uart-putc uart ch)))

(defn uart-read-line [uart]
  (letfn [(read-chars [acc]
            (let [ch (uart-getc uart)]
              (if (= ch (u8 \newline))
                (apply str (map char acc))
                (read-chars (conj acc ch)))))]
    (read-chars {})))
```

---

### Network Driver with Zero-Copy RX

```clojure
(ns drivers.virtio-net
  (:require [lona.io :as io]
            [lona.process :as proc]))

(def +RX-RING-SIZE+ 256u32)
(def +BUFFER-SIZE+ 2048u64)

(defn init-rx-buffers []
  (vec (for [_ (range +RX-RING-SIZE+)]
         (io/dma-alloc +BUFFER-SIZE+ :dma/uncached))))

(defn init-driver [paddr irq-num]
  (let [regs     (io/mmio-map paddr 0x1000u64 :device)
        ntfn     (proc/make-notification)
        irq      (io/irq-register! irq-num ntfn)
        rx-bufs  (init-rx-buffers)
        rx-ring  (io/ring-create +RX-RING-SIZE+ 16u32)
        tx-ring  (io/ring-create +RX-RING-SIZE+ 16u32)]

    ;; Populate RX ring with buffer descriptors
    (doseq [buf rx-bufs]
      (io/ring-produce! rx-ring
        %{:paddr (io/dma-paddr buf)
          :len   +BUFFER-SIZE+
          :flags 0u8}))

    %{:regs regs :irq irq :ntfn ntfn
      :rx-ring rx-ring :tx-ring tx-ring
      :rx-bufs rx-bufs}))

(defn rx-handler [%{:keys [ntfn irq rx-ring rx-bufs]} callback]
  (letfn [(process-descriptors []
            (when-let [desc (io/ring-consume rx-ring)]
              (let [buf (nth rx-bufs (:idx desc))
                    len (:actual-len desc)]
                ;; Sync for CPU access
                (io/dma-sync-for-cpu! buf 0u64 len)
                ;; Call user callback with packet data
                (callback %{:data (io/dma-vaddr buf) :len len})
                ;; Return buffer to ring
                (io/ring-produce! rx-ring
                  %{:paddr (io/dma-paddr buf)
                    :len   +BUFFER-SIZE+
                    :flags 0u8}))
              (process-descriptors)))
          (handler-loop []
            (io/irq-wait ntfn)
            (process-descriptors)
            (io/irq-ack! irq)
            (handler-loop))]
    (handler-loop)))

(defn tx-packet [%{:keys [tx-ring]} data]
  (let [buf (io/dma-alloc (count data) :dma/write-combine)]
    ;; Copy data to DMA buffer
    (dotimes [i (count data)]
      (io/mmio-write8! (vaddr+ (io/dma-vaddr buf) i) (nth data i)))
    ;; Sync for device access
    (io/dma-sync-for-device! buf 0u64 (count data))
    ;; Submit to TX ring
    (io/ring-produce! tx-ring
      %{:paddr (io/dma-paddr buf)
        :len   (count data)
        :flags 0u8})))
```

---

### Block Device Driver

```clojure
(ns drivers.virtio-blk
  (:require [lona.io :as io]
            [lona.process :as proc]))

(defn- wait-complete [regs]
  (if (zero? (io/mmio-read32 (vaddr+ regs 0x18u64)))
    (wait-complete regs)
    :done))

(defn read-sector [%{:keys [regs]} sector-num buf]
  (let [dma-buf (io/dma-alloc 512u64 :dma/cached)]
    ;; Setup read command
    (io/mmio-write64! (vaddr+ regs 0x00u64) sector-num)
    (io/mmio-write64! (vaddr+ regs 0x08u64) (paddr->u64 (io/dma-paddr dma-buf)))
    (io/mmio-write32! (vaddr+ regs 0x10u64) 512u32)
    (io/mmio-write32! (vaddr+ regs 0x14u64) 0u32)  ; Read command

    ;; Wait for completion
    (wait-complete regs)

    ;; Sync and copy data
    (io/dma-sync-for-cpu! dma-buf 0u64 512u64)
    (dotimes [i 512]
      (aset buf i (io/mmio-read8 (vaddr+ (io/dma-vaddr dma-buf) i))))

    (io/dma-free! dma-buf)
    :ok))

(defn write-sector [%{:keys [regs]} sector-num data]
  (let [dma-buf (io/dma-alloc 512u64 :dma/cached)]
    ;; Copy data to DMA buffer
    (dotimes [i 512]
      (io/mmio-write8! (vaddr+ (io/dma-vaddr dma-buf) i) (nth data i)))
    (io/dma-sync-for-device! dma-buf 0u64 512u64)

    ;; Setup write command
    (io/mmio-write64! (vaddr+ regs 0x00u64) sector-num)
    (io/mmio-write64! (vaddr+ regs 0x08u64) (paddr->u64 (io/dma-paddr dma-buf)))
    (io/mmio-write32! (vaddr+ regs 0x10u64) 512u32)
    (io/mmio-write32! (vaddr+ regs 0x14u64) 1u32)  ; Write command

    ;; Wait for completion
    (wait-complete regs)

    (io/dma-free! dma-buf)
    :ok))
```
