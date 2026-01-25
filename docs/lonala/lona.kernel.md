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

```clojure
;; @todo @x86_64
(arch)  ; => :x86_64
```

```clojure
;; @todo @aarch64
(arch)  ; => :aarch64
```

```clojure
;; @todo
(keyword? (arch))  ; => true
;; arch returns one of the supported architectures
(or (= (arch) :x86_64) (= (arch) :aarch64))  ; => true
```

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

```clojure
;; @todo
;; Non-blocking send returns immediately
(def ep endpoint)  ; assume endpoint exists
(def mi (msg-info 0 0 0))
(send-nb! ep mi)  ; => :ok
```

### `recv!`

Blocking receive.

```clojure
(recv! endpoint)  ; → [msg-info badge]
```

```clojure
;; @todo
;; recv! returns a tuple
(def result (recv! endpoint))
(tuple? result)  ; => true
```

### `recv-nb!`

Non-blocking receive.

```clojure
(recv-nb! endpoint)  ; → [msg-info badge] or nil
```

```clojure
;; @todo
;; Non-blocking receive returns nil if no message
(def result (recv-nb! endpoint))
(or (nil? result) (tuple? result))  ; => true
```

### `call!`

RPC: send + receive.

```clojure
(call! endpoint msg-info)  ; → [reply-info badge]
```

```clojure
;; @todo
;; call! returns a tuple
(def mi (msg-info 0 0 0))
(def result (call! endpoint mi))
(tuple? result)  ; => true
```

### `reply!`

Reply to caller.

```clojure
(reply! msg-info)  ; Reply to last call
```

```clojure
;; @todo
(def mi (msg-info 0 0 0))
(reply! mi)  ; => :ok
```

### `reply-recv!`

Reply + receive (server fast-path).

```clojure
(reply-recv! endpoint msg-info)  ; → [msg-info badge]
```

```clojure
;; @todo
(def mi (msg-info 0 0 0))
(def result (reply-recv! endpoint mi))
(tuple? result)  ; => true
```

### `yield!`

Voluntarily give up the current timeslice, allowing other threads to run.

```clojure
(yield!)
```

```clojure
;; @todo
(yield!)  ; => nil
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

```clojure
;; @todo
(def n (make-notification))
(signal! n)  ; => :ok
```

### `wait!`

Wait for notification.

```clojure
(wait! notification)  ; → badge
```

Blocks until signaled, returns and clears word.

```clojure
;; @todo
;; wait! returns an integer badge
(def n (make-notification))
(signal! n)
(integer? (wait! n))  ; => true
```

### `poll!`

Poll notification.

```clojure
(poll! notification)  ; → badge or nil
```

```clojure
;; @todo
(def n (make-notification))
(poll! n)  ; => nil

(signal! n)
(integer? (poll! n))  ; => true
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

```clojure
;; @todo
(tcb-configure! tcb %{:cspace-root cnode :vspace-root vspace})  ; => :ok
```

### `tcb-set-space!`

Set TCB's CSpace and VSpace.

```clojure
(tcb-set-space! tcb opts)
```

```clojure
;; @todo
(tcb-set-space! tcb %{:cspace-root cnode :vspace-root vspace})  ; => :ok
```

### `tcb-set-ipc-buffer!`

Set IPC buffer.

```clojure
(tcb-set-ipc-buffer! tcb frame vaddr)
```

```clojure
;; @todo
(tcb-set-ipc-buffer! tcb frame (vaddr 0x4000u64))  ; => :ok
```

### `tcb-set-priority!`

Set priority (0-255).

```clojure
(tcb-set-priority! tcb authority priority)
```

```clojure
;; @todo
(tcb-set-priority! tcb auth 100)  ; => :ok
```

### `tcb-set-mc-priority!`

Set max controlled priority.

```clojure
(tcb-set-mc-priority! tcb authority mcp)
```

```clojure
;; @todo
(tcb-set-mc-priority! tcb auth 200)  ; => :ok
```

### `tcb-set-sched-params!`

Set scheduling parameters (MCS).

```clojure
(tcb-set-sched-params! tcb authority opts)
```

Options: `:priority`, `:mcp`, `:sched-context`, `:fault-ep`.

```clojure
;; @todo
(tcb-set-sched-params! tcb auth %{:priority 100 :mcp 200})  ; => :ok
```

### `tcb-set-affinity!`

Pin to CPU core.

```clojure
(tcb-set-affinity! tcb core-id)
```

```clojure
;; @todo
(tcb-set-affinity! tcb 0)  ; => :ok
```

### `tcb-resume!`

Make thread runnable.

```clojure
(tcb-resume! tcb)
```

```clojure
;; @todo
(tcb-resume! tcb)  ; => :ok
```

### `tcb-suspend!`

Pause thread.

```clojure
(tcb-suspend! tcb)
```

```clojure
;; @todo
(tcb-suspend! tcb)  ; => :ok
```

### `tcb-read-regs`

Read registers from suspended thread.

```clojure
(tcb-read-regs tcb count suspend?)  ; → %{:pc :sp :regs [...]}
```

```clojure
;; @todo
(def regs (tcb-read-regs tcb 10 false))
(map? regs)            ; => true
(contains? regs :pc)   ; => true
(contains? regs :sp)   ; => true
(contains? regs :regs) ; => true
```

### `tcb-write-regs!`

Write registers.

```clojure
(tcb-write-regs! tcb resume? flags regs)
```

```clojure
;; @todo
(tcb-write-regs! tcb false 0 %{:pc 0x1000u64 :sp 0x2000u64})  ; => :ok
```

### `tcb-bind-notification!`

Bind notification to TCB.

```clojure
(tcb-bind-notification! tcb notification)
```

```clojure
;; @todo
(def n (make-notification))
(tcb-bind-notification! tcb n)  ; => :ok
```

### `tcb-unbind-notification!`

Unbind notification.

```clojure
(tcb-unbind-notification! tcb)
```

```clojure
;; @todo
(tcb-unbind-notification! tcb)  ; => :ok
```

---

## CNode Operations

Capability space management.

### `cap-copy!`

Copy capability with rights mask.

```clojure
(cap-copy! dest-cn dest-slot src-cn src-slot rights)
```

```clojure
;; @todo
(cap-copy! dest-cn 1 src-cn 2 #{:read})  ; => :ok
```

### `cap-mint!`

Copy with badge.

```clojure
(cap-mint! dest-cn dest-slot src-cn src-slot rights badge)
```

```clojure
;; @todo
(cap-mint! dest-cn 1 src-cn 2 #{:read :write} 0x1234)  ; => :ok
```

### `cap-move!`

Move capability.

```clojure
(cap-move! dest-cn dest-slot src-cn src-slot)
```

```clojure
;; @todo
(cap-move! dest-cn 1 src-cn 2)  ; => :ok
```

### `cap-mutate!`

Move with modified guard.

```clojure
(cap-mutate! dest-cn dest-slot src-cn src-slot guard)
```

```clojure
;; @todo
(cap-mutate! dest-cn 1 src-cn 2 0)  ; => :ok
```

### `cap-delete!`

Delete capability.

```clojure
(cap-delete! cnode slot)
```

```clojure
;; @todo
(cap-delete! cnode 5)  ; => :ok
```

### `cap-revoke!`

Delete capability and all derivatives.

```clojure
(cap-revoke! cnode slot)
```

```clojure
;; @todo
(cap-revoke! cnode 5)  ; => :ok
```

### `cap-rotate!`

Atomic three-way move.

```clojure
(cap-rotate! dest-cn dest-slot pivot-cn pivot-slot src-cn src-slot badge)
```

```clojure
;; @todo
(cap-rotate! dest-cn 1 pivot-cn 2 src-cn 3 0x5678)  ; => :ok
```

### `cap-save-caller!`

Save reply capability.

```clojure
(cap-save-caller! cnode slot)
```

```clojure
;; @todo
(cap-save-caller! cnode 10)  ; => :ok
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

```clojure
;; @todo
(untyped-retype! untyped :endpoint 0 dest-cn 0 1)  ; => :ok
```

```clojure
;; @todo
(untyped-retype! untyped :frame-4k 12 dest-cn 0 4)  ; => :ok
```

---

## Scheduling Operations (MCS)

### `sched-configure!`

Configure scheduling context.

```clojure
(sched-configure! sched-control sc opts)
```

Options: `:budget`, `:period`, `:extra-refills`, `:badge`, `:flags`.

```clojure
;; @todo
(sched-configure! sched-control sc %{:budget 100000 :period 1000000})  ; => :ok
```

### `sched-context-bind!`

Bind TCB or notification.

```clojure
(sched-context-bind! sc object)
```

```clojure
;; @todo
(sched-context-bind! sc tcb)  ; => :ok
```

### `sched-context-unbind!`

Unbind all objects.

```clojure
(sched-context-unbind! sc)
```

```clojure
;; @todo
(sched-context-unbind! sc)  ; => :ok
```

### `sched-context-unbind-object!`

Unbind specific object.

```clojure
(sched-context-unbind-object! sc object)
```

```clojure
;; @todo
(sched-context-unbind-object! sc tcb)  ; => :ok
```

### `sched-context-consumed`

Get consumed CPU time.

```clojure
(sched-context-consumed sc)  ; → nanoseconds
```

```clojure
;; @todo
(integer? (sched-context-consumed sc))  ; => true
(>= (sched-context-consumed sc) 0)      ; => true
```

---

## IRQ Operations

### `irq-control-get!`

Create IRQ handler capability.

```clojure
(irq-control-get! irq-control irq-num dest-cn dest-slot)
```

```clojure
;; @todo
(irq-control-get! irq-control 33 dest-cn 0)  ; => :ok
```

### `irq-handler-ack!`

Acknowledge interrupt.

```clojure
(irq-handler-ack! handler)
```

```clojure
;; @todo
(irq-handler-ack! handler)  ; => :ok
```

### `irq-handler-set-notification!`

Set notification for IRQ.

```clojure
(irq-handler-set-notification! handler notification)
```

```clojure
;; @todo
(def n (make-notification))
(irq-handler-set-notification! handler n)  ; => :ok
```

### `irq-handler-clear!`

Remove IRQ handler.

```clojure
(irq-handler-clear! handler)
```

```clojure
;; @todo
(irq-handler-clear! handler)  ; => :ok
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

```clojure
;; @todo
(page-map! frame vspace (vaddr 0x4000u64) #{:read :write} :cached)  ; => :ok
```

### `page-unmap!`

Unmap frame.

```clojure
(page-unmap! frame)
```

```clojure
;; @todo
(page-unmap! frame)  ; => :ok
```

### `page-get-address`

Get physical address.

```clojure
(page-get-address frame)  ; → paddr
```

```clojure
;; @todo
(paddr? (page-get-address frame))  ; => true
```

### `page-table-map!`

Map page table.

```clojure
(page-table-map! pt vspace vaddr attrs)
```

```clojure
;; @todo
(page-table-map! pt vspace (vaddr 0x200000u64) :cached)  ; => :ok
```

### `page-table-unmap!`

Unmap page table.

```clojure
(page-table-unmap! pt)
```

```clojure
;; @todo
(page-table-unmap! pt)  ; => :ok
```

### `asid-pool-assign!`

Assign ASID to VSpace.

```clojure
(asid-pool-assign! pool vspace)
```

```clojure
;; @todo
(asid-pool-assign! pool vspace)  ; => :ok
```

---

## Debug Operations

Available when kernel built with DEBUG_BUILD.

### `debug-put-char!`

Output to kernel serial.

```clojure
(debug-put-char! ch)
```

```clojure
;; @todo
(debug-put-char! \A)  ; => :ok
```

### `debug-halt!`

Halt system.

```clojure
(debug-halt!)
```

```clojure
;; @todo
;; debug-halt! halts the entire system - cannot test interactively
;; (debug-halt!)  ; Would halt system
```

### `debug-dump-scheduler!`

Dump scheduler state.

```clojure
(debug-dump-scheduler!)
```

```clojure
;; @todo
(debug-dump-scheduler!)  ; => :ok
```

### `debug-cap-identify`

Identify capability type.

```clojure
(debug-cap-identify cap)  ; → type keyword
```

```clojure
;; @todo
(keyword? (debug-cap-identify frame))  ; => true
(debug-cap-identify frame)             ; => :frame
```

### `debug-name-thread!`

Set thread name.

```clojure
(debug-name-thread! tcb name)
```

```clojure
;; @todo
(debug-name-thread! tcb "worker-1")  ; => :ok
```

---

## Appendix: Expected Derived Functions

All functions in this namespace are intrinsics. No derived functions expected.
