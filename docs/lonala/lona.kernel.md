# lona.kernel

Low-level seL4 kernel intrinsics. Used by VM runtime and system code, not application developers.

---

## System Info

### `arch`

Get current CPU architecture.

```clojure
(arch)  ; → :x86_64 or :aarch64
```

Use for architecture-specific code paths (e.g., selecting page table types).

---

## IPC Operations

seL4 IPC is synchronous and uses endpoints.

### `send!`

Blocking send.

```clojure
(send! endpoint msg-info)  ; Blocks until received
```

### `send-nb!`

Non-blocking send.

```clojure
(send-nb! endpoint msg-info)  ; Returns immediately
```

Message dropped if no receiver.

### `recv!`

Blocking receive.

```clojure
(recv! endpoint)  ; → [msg-info badge]
```

### `recv-nb!`

Non-blocking receive.

```clojure
(recv-nb! endpoint)  ; → [msg-info badge] or nil
```

### `call!`

RPC: send + receive.

```clojure
(call! endpoint msg-info)  ; → [reply-info badge]
```

### `reply!`

Reply to caller.

```clojure
(reply! msg-info)  ; Reply to last call
```

### `reply-recv!`

Reply + receive (server fast-path).

```clojure
(reply-recv! endpoint msg-info)  ; → [msg-info badge]
```

### `yield!`

Voluntarily give up the current timeslice, allowing other threads to run.

```clojure
(yield!)
```

**Implementation note:** Currently implemented via seL4's `seL4_SchedContext_YieldTo`.
This is an implementation detail and may change; do not rely on specific yield-to
semantics.

---

## Notification Operations

Asynchronous signaling.

### `signal!`

Signal notification.

```clojure
(signal! notification)
```

ORs badge into notification word, wakes waiters.

### `wait!`

Wait for notification.

```clojure
(wait! notification)  ; → badge
```

Blocks until signaled, returns and clears word.

### `poll!`

Poll notification.

```clojure
(poll! notification)  ; → badge or nil
```

---

## TCB Operations

Thread Control Block management.

### `tcb-configure!`

Configure TCB.

```clojure
(tcb-configure! tcb opts)
```

Options: `:fault-ep`, `:cspace-root`, `:cspace-data`, `:vspace-root`, `:vspace-data`, `:ipc-buffer`, `:ipc-buffer-addr`.

### `tcb-set-space!`

Set TCB's CSpace and VSpace.

```clojure
(tcb-set-space! tcb opts)
```

### `tcb-set-ipc-buffer!`

Set IPC buffer.

```clojure
(tcb-set-ipc-buffer! tcb frame vaddr)
```

### `tcb-set-priority!`

Set priority (0-255).

```clojure
(tcb-set-priority! tcb authority priority)
```

### `tcb-set-mc-priority!`

Set max controlled priority.

```clojure
(tcb-set-mc-priority! tcb authority mcp)
```

### `tcb-set-sched-params!`

Set scheduling parameters (MCS).

```clojure
(tcb-set-sched-params! tcb authority opts)
```

Options: `:priority`, `:mcp`, `:sched-context`, `:fault-ep`.

### `tcb-set-affinity!`

Pin to CPU core.

```clojure
(tcb-set-affinity! tcb core-id)
```

### `tcb-resume!`

Make thread runnable.

```clojure
(tcb-resume! tcb)
```

### `tcb-suspend!`

Pause thread.

```clojure
(tcb-suspend! tcb)
```

### `tcb-read-regs`

Read registers from suspended thread.

```clojure
(tcb-read-regs tcb count suspend?)  ; → %{:pc :sp :regs [...]}
```

### `tcb-write-regs!`

Write registers.

```clojure
(tcb-write-regs! tcb resume? flags regs)
```

### `tcb-bind-notification!`

Bind notification to TCB.

```clojure
(tcb-bind-notification! tcb notification)
```

### `tcb-unbind-notification!`

Unbind notification.

```clojure
(tcb-unbind-notification! tcb)
```

---

## CNode Operations

Capability space management.

### `cap-copy!`

Copy capability with rights mask.

```clojure
(cap-copy! dest-cn dest-slot src-cn src-slot rights)
```

### `cap-mint!`

Copy with badge.

```clojure
(cap-mint! dest-cn dest-slot src-cn src-slot rights badge)
```

### `cap-move!`

Move capability.

```clojure
(cap-move! dest-cn dest-slot src-cn src-slot)
```

### `cap-mutate!`

Move with modified guard.

```clojure
(cap-mutate! dest-cn dest-slot src-cn src-slot guard)
```

### `cap-delete!`

Delete capability.

```clojure
(cap-delete! cnode slot)
```

### `cap-revoke!`

Delete capability and all derivatives.

```clojure
(cap-revoke! cnode slot)
```

### `cap-rotate!`

Atomic three-way move.

```clojure
(cap-rotate! dest-cn dest-slot pivot-cn pivot-slot src-cn src-slot badge)
```

### `cap-save-caller!`

Save reply capability.

```clojure
(cap-save-caller! cnode slot)
```

---

## Untyped Operations

### `untyped-retype!`

Convert untyped to typed objects.

```clojure
(untyped-retype! untyped type size-bits dest-cn dest-slot count)
```

**Common types:**
`:tcb`, `:endpoint`, `:notification`, `:cnode`, `:sched-context`

**Frame types:**
`:frame-4k`, `:frame-2m`, `:frame-1g`

**Page table types (architecture-specific):**

| x86_64 | ARM64 (aarch64) | Description |
|--------|-----------------|-------------|
| `:pml4` | `:pgd` | Root page table (VSpace root) |
| `:pdpt` | `:pud` | Level 2 |
| `:page-directory` | `:pmd` | Level 3 |
| `:page-table` | `:pte` | Level 4 (leaf) |

Use `(arch)` to detect architecture and select appropriate types:

```clojure
(def vspace-root-type
  (match (arch)
    :x86_64  :pml4
    :aarch64 :pgd))
```

---

## Scheduling Operations (MCS)

### `sched-configure!`

Configure scheduling context.

```clojure
(sched-configure! sched-control sc opts)
```

Options: `:budget`, `:period`, `:extra-refills`, `:badge`, `:flags`.

### `sched-context-bind!`

Bind TCB or notification.

```clojure
(sched-context-bind! sc object)
```

### `sched-context-unbind!`

Unbind all objects.

```clojure
(sched-context-unbind! sc)
```

### `sched-context-unbind-object!`

Unbind specific object.

```clojure
(sched-context-unbind-object! sc object)
```

### `sched-context-consumed`

Get consumed CPU time.

```clojure
(sched-context-consumed sc)  ; → nanoseconds
```

---

## IRQ Operations

### `irq-control-get!`

Create IRQ handler capability.

```clojure
(irq-control-get! irq-control irq-num dest-cn dest-slot)
```

### `irq-handler-ack!`

Acknowledge interrupt.

```clojure
(irq-handler-ack! handler)
```

### `irq-handler-set-notification!`

Set notification for IRQ.

```clojure
(irq-handler-set-notification! handler notification)
```

### `irq-handler-clear!`

Remove IRQ handler.

```clojure
(irq-handler-clear! handler)
```

---

## VSpace Operations

### `page-map!`

Map frame into address space.

```clojure
(page-map! frame vspace vaddr rights attrs)
```

Rights: `:read`, `:write`, `:execute`

Attrs: `:cached`, `:uncached`, `:device`, `:write-combine`

### `page-unmap!`

Unmap frame.

```clojure
(page-unmap! frame)
```

### `page-get-address`

Get physical address.

```clojure
(page-get-address frame)  ; → paddr
```

### `page-table-map!`

Map page table.

```clojure
(page-table-map! pt vspace vaddr attrs)
```

### `page-table-unmap!`

Unmap page table.

```clojure
(page-table-unmap! pt)
```

### `asid-pool-assign!`

Assign ASID to VSpace.

```clojure
(asid-pool-assign! pool vspace)
```

---

## Debug Operations

Available when kernel built with DEBUG_BUILD.

### `debug-put-char!`

Output to kernel serial.

```clojure
(debug-put-char! ch)
```

### `debug-halt!`

Halt system.

```clojure
(debug-halt!)
```

### `debug-dump-scheduler!`

Dump scheduler state.

```clojure
(debug-dump-scheduler!)
```

### `debug-cap-identify`

Identify capability type.

```clojure
(debug-cap-identify cap)  ; → type keyword
```

### `debug-name-thread!`

Set thread name.

```clojure
(debug-name-thread! tcb name)
```

---

## Appendix: Expected Derived Functions

All functions in this namespace are intrinsics. No derived functions expected.
