# Lonala Kernel Specification

> **Namespace:** `lona.kernel`

This document specifies the low-level seL4 kernel primitives exposed to Lonala programs. These are primarily used by the VM runtime and system-level code, not application developers.

**Related:** [lonala.md](lonala.md) (core language) | [lonala-process.md](lonala-process.md) (process primitives) | [lonala-io.md](lonala-io.md) (device I/O)

---

## Table of Contents

1. [Overview](#overview)
2. [seL4 Background](#sel4-background)
3. [IPC Operations](#ipc-operations)
4. [Notification Operations](#notification-operations)
5. [TCB Operations](#tcb-operations)
6. [CNode Operations](#cnode-operations)
7. [Untyped Operations](#untyped-operations)
8. [Scheduling Operations](#scheduling-operations)
9. [IRQ Operations](#irq-operations)
10. [VSpace Operations](#vspace-operations)
11. [Debug Operations](#debug-operations)
12. [API Reference](#api-reference)

---

## Overview

The `lona.kernel` namespace provides direct access to seL4 system calls and object methods. These primitives are:

- **Low-level**: Direct mapping to seL4 operations
- **Capability-based**: All operations require appropriate capabilities
- **Unsafe**: Incorrect use can crash the realm or corrupt state
- **For VM implementers**: Not typically used by application code

The Lonala VM runtime uses these primitives to implement higher-level abstractions like `lona.process` and `lona.io`.

---

## seL4 Background

### Capabilities

seL4 uses capabilities for all access control. A capability is a token granting specific rights to a kernel object.

```
Capability = Reference + Rights

Example capabilities:
  - TCB cap with Write right → can modify thread
  - Endpoint cap with Send right → can send messages
  - Frame cap with Read right → can map read-only
```

### Kernel Objects

| Object | Purpose |
|--------|---------|
| TCB | Thread Control Block — schedulable entity |
| Endpoint | Synchronous IPC channel |
| Notification | Asynchronous signaling |
| CNode | Capability container |
| Untyped | Raw memory (can be retyped) |
| Frame | Physical memory page |
| VSpace | Virtual address space |
| SchedContext | CPU time budget (MCS) |

---

## IPC Operations

seL4 IPC is synchronous and uses endpoints.

### `send!`

Send message to endpoint (blocking).

```clojure
(send! endpoint msg-info)
```

Blocks until receiver is ready.

---

### `send-nb!`

Send message to endpoint (non-blocking).

```clojure
(send-nb! endpoint msg-info)
```

Returns immediately. Message dropped if no receiver ready.

---

### `recv!`

Receive message from endpoint (blocking).

```clojure
(recv! endpoint)  ; → [msg-info badge]
```

Blocks until sender ready.

---

### `recv-nb!`

Receive message (non-blocking).

```clojure
(recv-nb! endpoint)  ; → [msg-info badge] or nil
```

---

### `call!`

Combined send + receive (RPC pattern).

```clojure
(call! endpoint msg-info)  ; → [reply-info badge]
```

Sends message, blocks waiting for reply. Uses one-time reply capability.

---

### `reply!`

Reply to caller via reply capability.

```clojure
(reply! msg-info)
```

Sends reply to most recent `call!` caller.

---

### `reply-recv!`

Combined reply + receive (server fast-path).

```clojure
(reply-recv! endpoint msg-info)  ; → [next-msg-info badge]
```

Replies to previous caller and waits for next message in one syscall.

---

### `yield!`

Donate remaining timeslice.

```clojure
(yield!)
```

Yields to another thread of same priority.

---

## Notification Operations

Notifications provide asynchronous signaling via a word that can be OR'd.

### `signal!`

Signal a notification.

```clojure
(signal! notification)
```

OR's the capability's badge into the notification word. Wakes waiting threads.

---

### `wait!`

Wait for notification (blocking).

```clojure
(wait! notification)  ; → badge-word
```

Blocks until notification is signaled. Returns and clears the notification word.

---

### `poll!`

Poll notification (non-blocking).

```clojure
(poll! notification)  ; → badge-word or nil
```

Returns notification word if non-zero, nil otherwise.

---

## TCB Operations

Thread Control Block operations manage kernel threads.

### `tcb-configure!`

Configure a TCB with address spaces and fault endpoint.

```clojure
(tcb-configure! tcb
  %{:fault-ep fault-endpoint
    :cspace-root cnode
    :cspace-data guard-data
    :vspace-root vspace
    :vspace-data asid
    :ipc-buffer buffer-frame
    :ipc-buffer-addr vaddr})
```

---

### `tcb-set-space!`

Change TCB's CSpace and VSpace.

```clojure
(tcb-set-space! tcb
  %{:fault-ep fault-endpoint
    :cspace-root cnode
    :cspace-data guard-data
    :vspace-root vspace
    :vspace-data asid})
```

---

### `tcb-set-ipc-buffer!`

Set IPC buffer location.

```clojure
(tcb-set-ipc-buffer! tcb buffer-frame vaddr)
```

---

### `tcb-set-priority!`

Set thread priority (0-255).

```clojure
(tcb-set-priority! tcb authority priority)
```

---

### `tcb-set-mc-priority!`

Set maximum controlled priority.

```clojure
(tcb-set-mc-priority! tcb authority mcp)
```

---

### `tcb-set-sched-params!`

Set priority, MCP, and bind scheduling context (MCS).

```clojure
(tcb-set-sched-params! tcb authority
  %{:priority priority
    :mcp mcp
    :sched-context sc
    :fault-ep fault-ep})
```

---

### `tcb-set-affinity!`

Pin thread to specific CPU core.

```clojure
(tcb-set-affinity! tcb core-id)
```

---

### `tcb-resume!`

Make thread runnable.

```clojure
(tcb-resume! tcb)
```

---

### `tcb-suspend!`

Pause thread.

```clojure
(tcb-suspend! tcb)
```

---

### `tcb-read-regs`

Read CPU registers from suspended thread.

```clojure
(tcb-read-regs tcb count suspend?)
;; → %{:pc :sp :regs [...]}
```

---

### `tcb-write-regs!`

Write CPU registers.

```clojure
(tcb-write-regs! tcb resume? flags regs)
```

---

### `tcb-bind-notification!`

Bind notification to TCB for combined waiting.

```clojure
(tcb-bind-notification! tcb notification)
```

---

### `tcb-unbind-notification!`

Unbind notification from TCB.

```clojure
(tcb-unbind-notification! tcb)
```

---

## CNode Operations

CNode operations manage the capability space.

### `cap-copy!`

Copy capability with rights mask.

```clojure
(cap-copy! dest-cnode dest-slot src-cnode src-slot rights)
```

**Rights:** `:read`, `:write`, `:grant`, `:grant-reply`

---

### `cap-mint!`

Copy capability with badge.

```clojure
(cap-mint! dest-cnode dest-slot src-cnode src-slot rights badge)
```

Creates badged capability. Badge is transferred in IPC.

---

### `cap-move!`

Move capability to new slot.

```clojure
(cap-move! dest-cnode dest-slot src-cnode src-slot)
```

Source slot becomes empty.

---

### `cap-mutate!`

Move capability with modified guard.

```clojure
(cap-mutate! dest-cnode dest-slot src-cnode src-slot guard)
```

---

### `cap-delete!`

Delete capability from slot.

```clojure
(cap-delete! cnode slot)
```

---

### `cap-revoke!`

Delete capability and all derived capabilities.

```clojure
(cap-revoke! cnode slot)
```

Recursively deletes all capabilities derived from this one.

---

### `cap-rotate!`

Atomic three-way capability move.

```clojure
(cap-rotate! dest-cnode dest-slot pivot-cnode pivot-slot
             src-cnode src-slot pivot-badge)
```

---

### `cap-save-caller!`

Save reply capability from last call.

```clojure
(cap-save-caller! cnode slot)
```

---

## Untyped Operations

Untyped memory is raw physical memory that can be converted to typed objects.

### `untyped-retype!`

Convert untyped memory to typed objects.

```clojure
(untyped-retype! untyped type size-bits dest-cnode dest-slot num-objects)
```

**Types:**
| Type | Description |
|------|-------------|
| `:tcb` | Thread Control Block |
| `:endpoint` | IPC Endpoint |
| `:notification` | Notification object |
| `:cnode` | Capability Node |
| `:frame-4k` | 4KB page frame |
| `:frame-2m` | 2MB large page (x86) |
| `:frame-1g` | 1GB huge page (x86) |
| `:page-table` | Page table structure |
| `:vspace` | Virtual address space root |
| `:sched-context` | Scheduling context (MCS) |

**Example:**
```clojure
;; Create 4 TCBs from untyped memory
(untyped-retype! untyped :tcb 12 dest-cnode 0 4)
```

---

## Scheduling Operations

MCS (Mixed-Criticality Scheduling) operations manage CPU time budgets.

### `sched-configure!`

Configure scheduling context with budget and period.

```clojure
(sched-configure! sched-control sched-context
  %{:budget budget-ns
    :period period-ns
    :extra-refills n
    :badge badge
    :flags flags})
```

**Flags:**
- `:sporadic` — Sporadic server scheduling
- `:constant-bandwidth` — Constant bandwidth (default)

**Example:**
```clojure
;; 30% CPU: 300ms budget per 1s period
(sched-configure! sched-control sc
  %{:budget  300000000u64   ; 300ms in nanoseconds
    :period 1000000000u64   ; 1s
    :flags #{}})
```

---

### `sched-context-bind!`

Bind TCB or notification to scheduling context.

```clojure
(sched-context-bind! sched-context tcb-or-notification)
```

---

### `sched-context-unbind!`

Unbind all objects from scheduling context.

```clojure
(sched-context-unbind! sched-context)
```

---

### `sched-context-unbind-object!`

Unbind specific object.

```clojure
(sched-context-unbind-object! sched-context object)
```

---

### `sched-context-consumed`

Get consumed CPU time.

```clojure
(sched-context-consumed sched-context)  ; → nanoseconds
```

---

### `sched-yield-to!`

Yield remaining timeslice to a specific scheduling context. The TCB bound to the target SchedContext will run with the yielded time.

```clojure
(sched-yield-to! sched-context)
```

**Note:** Maps to `seL4_SchedContext_YieldTo`. Yields to whatever TCB is currently bound to the given SchedContext.

---

## IRQ Operations

Interrupt handling primitives.

### `irq-control-get!`

Create IRQ handler capability.

```clojure
(irq-control-get! irq-control irq-num dest-cnode dest-slot)
```

---

### `irq-handler-ack!`

Acknowledge interrupt (re-enable).

```clojure
(irq-handler-ack! irq-handler)
```

---

### `irq-handler-set-notification!`

Set notification for IRQ delivery.

```clojure
(irq-handler-set-notification! irq-handler notification)
```

---

### `irq-handler-clear!`

Remove IRQ handler.

```clojure
(irq-handler-clear! irq-handler)
```

---

## VSpace Operations

Virtual address space management.

### `page-map!`

Map frame into address space.

```clojure
(page-map! frame vspace vaddr rights attrs)
```

**Rights:** `:read`, `:write`, `:execute`

**Attributes:**
| Attribute | Description |
|-----------|-------------|
| `:cached` | Normal cached memory (default) |
| `:uncached` | No caching |
| `:device` | Device memory (strongly ordered, for MMIO) |
| `:write-combine` | Write-combining (frame buffers, TX buffers) |

---

### `page-unmap!`

Unmap frame from address space.

```clojure
(page-unmap! frame)
```

---

### `page-get-address`

Get physical address of frame.

```clojure
(page-get-address frame)  ; → paddr
```

---

### `page-table-map!`

Map page table structure.

```clojure
(page-table-map! page-table vspace vaddr attrs)
```

---

### `page-table-unmap!`

Unmap page table.

```clojure
(page-table-unmap! page-table)
```

---

### `asid-pool-assign!`

Assign ASID to VSpace.

```clojure
(asid-pool-assign! asid-pool vspace)
```

---

## Debug Operations

Available when kernel built with DEBUG_BUILD.

### `debug-put-char!`

Output character to kernel serial.

```clojure
(debug-put-char! ch)
```

---

### `debug-halt!`

Halt the system.

```clojure
(debug-halt!)
```

---

### `debug-dump-scheduler!`

Dump scheduler state.

```clojure
(debug-dump-scheduler!)
```

---

### `debug-cap-identify`

Identify capability type.

```clojure
(debug-cap-identify cap)  ; → type keyword
```

---

### `debug-name-thread!`

Set thread name for debugging.

```clojure
(debug-name-thread! tcb name)
```

---

## API Reference

### IPC

```clojure
(send! endpoint msg-info)
(send-nb! endpoint msg-info)
(recv! endpoint)                    ; → [msg-info badge]
(recv-nb! endpoint)                 ; → [msg-info badge] or nil
(call! endpoint msg-info)           ; → [reply-info badge]
(reply! msg-info)
(reply-recv! endpoint msg-info)     ; → [msg-info badge]
(yield!)
```

### Notifications

```clojure
(signal! notification)
(wait! notification)                ; → badge
(poll! notification)                ; → badge or nil
```

### TCB

```clojure
(tcb-configure! tcb opts)
(tcb-set-space! tcb opts)
(tcb-set-ipc-buffer! tcb frame vaddr)
(tcb-set-priority! tcb auth priority)
(tcb-set-mc-priority! tcb auth mcp)
(tcb-set-sched-params! tcb auth opts)
(tcb-set-affinity! tcb core)
(tcb-resume! tcb)
(tcb-suspend! tcb)
(tcb-read-regs tcb count suspend?)  ; → %{:pc :sp :regs [...]}
(tcb-write-regs! tcb resume? flags regs)
(tcb-bind-notification! tcb ntfn)
(tcb-unbind-notification! tcb)
```

### CNode

```clojure
(cap-copy! dest-cn dest-slot src-cn src-slot rights)
(cap-mint! dest-cn dest-slot src-cn src-slot rights badge)
(cap-move! dest-cn dest-slot src-cn src-slot)
(cap-mutate! dest-cn dest-slot src-cn src-slot guard)
(cap-delete! cnode slot)
(cap-revoke! cnode slot)
(cap-rotate! dest-cn dest-slot pivot-cn pivot-slot src-cn src-slot badge)
(cap-save-caller! cnode slot)
```

### Untyped

```clojure
(untyped-retype! untyped type size-bits dest-cn dest-slot count)
```

### Scheduling (MCS)

```clojure
(sched-configure! sched-ctrl sc opts)
(sched-context-bind! sc object)
(sched-context-unbind! sc)
(sched-context-unbind-object! sc object)
(sched-context-consumed sc)         ; → nanoseconds
(sched-yield-to! sc)
```

### IRQ

```clojure
(irq-control-get! irq-ctrl irq-num dest-cn dest-slot)
(irq-handler-ack! handler)
(irq-handler-set-notification! handler ntfn)
(irq-handler-clear! handler)
```

### VSpace

```clojure
(page-map! frame vspace vaddr rights attrs)
(page-unmap! frame)
(page-get-address frame)            ; → paddr
(page-table-map! pt vspace vaddr attrs)
(page-table-unmap! pt)
(asid-pool-assign! pool vspace)
```

### Debug

```clojure
(debug-put-char! ch)
(debug-halt!)
(debug-dump-scheduler!)
(debug-cap-identify cap)            ; → type
(debug-name-thread! tcb name)
```
