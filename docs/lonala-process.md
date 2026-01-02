# Lonala Process Specification

> **Namespace:** `lona.process`

This document specifies process and realm primitives for Lonala, providing BEAM-style lightweight processes with message passing, fault tolerance, and hierarchical realm management.

**Related:** [lonala.md](lonala.md) (core language) | [lonala-kernel.md](lonala-kernel.md) (kernel primitives) | [lonala-io.md](lonala-io.md) (device I/O)

---

## Table of Contents

1. [Overview](#overview)
2. [Process Identifiers (PIDs)](#process-identifiers-pids)
3. [Process Characteristics](#process-characteristics)
4. [Process Creation](#process-creation)
5. [Process Identity](#process-identity)
6. [Message Passing](#message-passing)
7. [Receive with Pattern Matching](#receive-with-pattern-matching)
8. [Process Linking](#process-linking)
9. [Process Monitoring](#process-monitoring)
10. [Process Lifecycle](#process-lifecycle)
11. [Process Registry](#process-registry)
12. [Realm Management](#realm-management)
13. [Cross-Realm Process Creation](#cross-realm-process-creation)
14. [Shared Memory](#shared-memory)
15. [Resource Management](#resource-management)
16. [Supervisors](#supervisors)
17. [API Reference](#api-reference)

---

## Overview

Lonala provides two levels of isolation:

**Processes** — Lightweight execution units within a realm:
- ~512 bytes minimum, millions per realm
- Isolated heaps, no shared mutable state
- Message passing communication
- Pure userspace construct (no kernel objects)

**Realms** — Protection domains with hardware-enforced isolation:
- Own VSpace (address space), CSpace (capabilities), SchedContext (CPU budget)
- Hierarchical: parent creates children, resources flow down
- Security boundary: compromised realm cannot affect siblings or parent
- Kernel-enforced resource limits

Processes are multiplexed onto realm scheduler TCBs. A realm can have millions of processes but typically only one TCB per CPU core.

---

## Process Identifiers (PIDs)

PIDs are **first-class types** containing both realm and local process identity:

```clojure
(pid realm-id local-id)   ; Constructor: e.g., (pid 3 42)
```

- `realm-id` — Integer identifying the realm
- `local-id` — Integer unique within that realm

**Accessors:**
```clojure
(pid-realm p)             ; → realm-id
(pid-local p)             ; → local-id
(pid? x)                  ; → boolean (type predicate)
(pid= p1 p2)              ; → boolean (equality)
```

**Properties:**
- All process functions accept and return `pid` values
- Routing is transparent: `send` works the same for local and remote
- PIDs are comparable, hashable, and can be pattern matched
- Pattern matching: `(pid r l)` destructures in `match` expressions

**Examples:**
```clojure
(self)                    ; → pid
(pid-realm (self))        ; → 5 (realm-id)
(pid-local (self))        ; → 17 (local-id)

(spawn (fn [] (work)))    ; → pid (same realm as caller)

;; Pattern matching PIDs
(match some-pid
  (pid r l) when (= r (self-realm))
    [:local l]
  (pid r l)
    [:remote r l])
```

---

## Process Characteristics

| Property | Value |
|----------|-------|
| Minimum size | ~512 bytes |
| Initial heap | 4 KB (configurable) |
| Kernel objects | None (pure userspace) |
| Creation time | ~1-10 µs |
| Max per realm | Millions |
| GC | Per-process, non-blocking |
| Communication | Message passing only |

---

## Process Creation

### `spawn`

Create a new process in the **current realm**.

```clojure
(spawn f)
(spawn f opts)
```

**Parameters:**
- `f` — Zero-argument function to execute
- `opts` — Optional map of process options

**Options:**
| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `:min-heap-size` | integer | 4096 | Initial heap size in bytes |
| `:max-heap-size` | integer | nil | Maximum heap size (nil = unlimited) |
| `:priority` | 0-255 | 100 | Scheduling priority |
| `:name` | symbol | nil | Register with local name |
| `:scheduler-affinity` | integer | nil | Pin to specific scheduler |

**Returns:** `pid`

**Examples:**
```clojure
;; Basic spawn
(spawn (fn [] (worker-loop)))
; → pid

;; With options
(spawn (fn [] (worker-loop))
  %{:min-heap-size 65536
    :priority 150
    :name 'my-worker})
; → pid
```

---

### `spawn-link`

Create a new process linked to the current process.

```clojure
(spawn-link f)
(spawn-link f opts)
```

If either process crashes, the other receives an exit signal. This creates **bidirectional** crash notification.

**Returns:** `pid`

**Example:**
```clojure
(spawn-link (fn [] (critical-task)))
; → pid
;; If critical-task crashes, current process receives [:EXIT crashed-pid reason]
;; If current process crashes, critical-task receives [:EXIT (self) reason]
```

---

### `spawn-monitor`

Create a new process monitored by the current process.

```clojure
(spawn-monitor f)
(spawn-monitor f opts)
```

**Unidirectional** crash notification — only the spawning process receives notifications.

**Returns:** `[pid monitor-ref]`

**Example:**
```clojure
(let [[pid mref] (spawn-monitor (fn [] (task)))]
  (receive
    [:DOWN mref pid reason] (handle-crash reason)))
```

---

## Process Identity

### `self`

Returns the current process's PID.

```clojure
(self)  ; → pid
```

### `self-realm`

Returns the current realm's ID. Convenience for `(pid-realm (self))`.

```clojure
(self-realm)  ; → realm-id
```

### `alive?`

Check if a process is still running. Works cross-realm (synchronous IPC if remote).

```clojure
(alive? pid)  ; → boolean
```

**Note:** For remote processes, this involves IPC and may have latency.

### `process-info`

Get information about a process. Works cross-realm.

```clojure
(process-info pid)
; → %{:status :running
;     :realm-id 5
;     :local-id 42
;     :heap-size 8192
;     :mailbox-len 3
;     :priority 100
;     :links #{pid ...}
;     :monitors #{monitor-ref ...}
;     ...}
```

Returns `nil` if process doesn't exist.

---

## Message Passing

### `send`

Send a message to a process asynchronously. Works uniformly for local and remote processes.

```clojure
(send pid message)
```

**Parameters:**
- `pid` — Target process (first-class `pid` type)
- `message` — Any Lonala value

**Returns:** `:ok` (always succeeds, fire-and-forget)

**Routing:**
- Same realm: Direct delivery, ~100-500 ns, deep-copy to receiver's heap (large binaries are reference-counted, not copied)
- Different realm: seL4 IPC, ~1-10 µs, full serialization (binaries cannot be shared across realms)

**Examples:**
```clojure
(send worker-pid [:task %{:id 42 :data [1 2 3]}])
(send logger-pid [:log :info "Processing complete"])

;; Works the same regardless of realm
(send worker-pid [:local-message])    ; Same realm
(send remote-pid [:remote-message])   ; Different realm, transparent IPC
```

**Semantics:**
- Message is deep-copied to receiver's heap (exception: large binaries within same realm use reference counting)
- Large binaries (>256 bytes) in same-realm messages share the underlying data via refcounted binary pool
- Inter-realm messages are fully serialized; binaries cannot be shared across realm boundaries (use shared regions instead)
- Delivery is asynchronous (non-blocking)
- Order preserved between same sender-receiver pair
- No delivery guarantee if receiver crashes

---

### `send!`

Send a message and wait for acknowledgment (synchronous).

```clojure
(send! pid message)
(send! pid message timeout-ms)
```

**Returns:** `:ok` or `:timeout`

---

### `send-named`

Send to a registered name. Performs hierarchical lookup then sends.

```clojure
(send-named name message)
```

**Returns:** `:ok` or `:not-found`

---

## Receive with Pattern Matching

### `receive`

Wait for and pattern-match incoming messages.

```clojure
(receive
  pattern1 body1
  pattern2 when guard body2
  ...
  :after timeout-ms timeout-body)
```

**Semantics:**
- Blocks until a matching message arrives (or timeout)
- Patterns are tried in order against mailbox messages
- Non-matching messages remain in mailbox (selective receive)
- Matching message is removed from mailbox
- `when` guards work like in `match`

**Examples:**

```clojure
;; Simple receive
(receive
  [:ok result] result
  [:error reason] (handle-error reason))

;; With guards
(receive
  [:data n] when (> n 0) (process-positive n)
  [:data n] (process-non-positive n))

;; With timeout
(receive
  [:response data] (handle-response data)
  :after 5000 (handle-timeout))

;; Handling exit signals (PIDs pattern matched)
(receive
  [:EXIT (pid r l) :normal] (log "Process in realm" r "exited normally")
  [:EXIT p reason] (log "Process" p "crashed:" reason)
  msg (handle-normal-message msg))

;; Catch-all
(receive
  [:expected msg] (handle-expected msg)
  other (log "Unexpected:" other))
```

---

### `receive-nb`

Non-blocking receive — returns immediately if no matching message.

```clojure
(receive-nb
  pattern1 body1
  ...
  :else no-match-body)
```

**Example:**
```clojure
(receive-nb
  [:urgent msg] (handle-urgent msg)
  :else :no-message)
```

---

## Process Linking

Links create bidirectional crash propagation between processes. Works cross-realm.

### `link`

Create a link between current process and another.

```clojure
(link pid)
```

**Example:**
```clojure
(link remote-pid)  ; Link to process in another realm
;; Now if remote-pid crashes, we get [:EXIT remote-pid reason]
;; If we crash, remote-pid gets [:EXIT (self) reason]
```

---

### `unlink`

Remove a link.

```clojure
(unlink pid)
```

---

### Exit Signals

When a linked process exits, linked processes receive:

```clojure
[:EXIT pid reason]
; e.g., [:EXIT some-pid :normal]
```

Where `reason` is:
- `:normal` — Process exited normally
- `:killed` — Process was killed
- `[:error type]` — Process crashed with error
- Any other term — Custom exit reason

**Cross-realm ordering guarantees:**
- From same source: Messages arrive before exit signal
- Across sources: No ordering guarantee

**Handling Exit Signals:**

```clojure
(receive
  [:EXIT pid :normal] (log "Process" pid "exited normally")
  [:EXIT pid reason] (log "Process" pid "crashed:" reason)
  msg (handle-normal-message msg))
```

---

### `trap-exit`

Enable trapping exit signals as messages instead of propagating crashes.

```clojure
(trap-exit true)   ; Enable trapping
(trap-exit false)  ; Disable trapping (default)
```

With `trap-exit` enabled, exit signals become regular messages instead of causing the receiving process to crash.

---

## Process Monitoring

Monitors provide **unidirectional** crash notification without crash propagation. Works cross-realm.

### `monitor`

Start monitoring a process.

```clojure
(monitor pid)  ; → monitor-ref
```

**Example:**
```clojure
(monitor remote-pid)  ; Monitor process in another realm
```

---

### `demonitor`

Stop monitoring a process.

```clojure
(demonitor monitor-ref)
```

---

### Monitor Messages

When a monitored process exits:

```clojure
[:DOWN monitor-ref pid reason]
; e.g., [:DOWN mref some-pid :normal]
```

**Example:**
```clojure
(let [mref (monitor target-pid)]
  (receive
    [:DOWN mref _ :normal] (log "Process finished")
    [:DOWN mref _ reason] (log "Process crashed:" reason)))
```

---

## Process Lifecycle

### `exit`

Terminate a process.

```clojure
(exit reason)           ; Exit current process
(exit pid reason)       ; Send exit signal to another process
```

**Examples:**
```clojure
(exit :normal)          ; Normal termination
(exit :shutdown)        ; Controlled shutdown
(exit [:error :timeout]) ; Error termination

;; Kill another process (works cross-realm)
(exit target-pid :kill)
```

---

### Exit Reasons

| Reason | Meaning |
|--------|---------|
| `:normal` | Clean exit, linked processes not affected |
| `:shutdown` | Controlled shutdown |
| `:kill` | Forceful termination (untrappable) |
| `[:error type]` | Error with type |
| Other | Custom reason, propagates to links |

---

## Process Registry

### Local Registry (within realm)

```clojure
;; Register current process
(register name)
(register name pid)

;; Unregister
(unregister name)

;; Lookup - local realm only
(whereis-local name)  ; → pid or nil
```

---

### Hierarchical Lookup

```clojure
(whereis name)
```

Searches registries in order:
1. Current realm's registry
2. Parent realm's registry
3. Grandparent's registry
4. ... up to root realm

**Returns:** `pid` or `nil`

**Example:**
```clojure
(register 'db-server)
;; Now other processes can:
(send-named 'db-server [:query "SELECT *"])

;; Child realms can also find it via hierarchical lookup
(whereis 'db-server)  ; → (pid 5 42) (found in parent)
```

---

## Realm Management

Realms are hierarchical protection domains. Resources are allocated **eagerly** at creation time.

### `realm-create`

Create a new child realm.

```clojure
(realm-create opts)
```

**Options:**
| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `:name` | symbol | yes | Realm name |
| `:policy` | map | no | Resource policy |
| `:schedulers` | `:auto` or integer | no | Scheduler count (default `:auto`) |
| `:shared` | set | no | Regions to map into new realm |

**Policy structure:**
```clojure
%{:cpu %{:min 0.1      ; Guaranteed minimum (reserved immediately)
         :max 0.3}     ; Maximum allowed (ceiling)
  :memory %{:min (* 100 +MB+)  ; Reserved immediately from parent's pool
            :max (* 500 +MB+)}} ; Maximum requestable
```

**Returns:** `realm-id`

**Resource allocation (eager):**
- CPU `:min` → SchedContext budget configured immediately
- Memory `:min` → Untyped capabilities granted immediately from parent's pool
- Scheduler TCBs created and started (idle, waiting for processes)

**Fails if:**
- Parent doesn't have memory `:min` available
- Parent doesn't have CPU `:min` budget to allocate
- Policy exceeds parent's own limits

**Examples:**
```clojure
;; Minimal realm (shares parent's budget)
(realm-create %{:name 'worker})

;; With resource policy
(realm-create %{:name 'database
                :policy %{:cpu %{:min 0.2 :max 0.5}
                          :memory %{:min (* 1 +GB+)
                                    :max (* 8 +GB+)}}})

;; With shared memory region
(let [data (make-shared-region (* 100 +MB+) 'dataset)]
  (realm-create %{:name 'processor
                  :shared #{data}}))
```

---

### `realm-terminate`

Terminate a realm and reclaim all resources.

```clojure
(realm-terminate realm-id)
```

**Process:**
1. Terminates all processes in the realm
2. Recursively terminates all child realms
3. Revokes all Untyped capabilities (memory returns to parent)
4. Destroys SchedContext (CPU budget returns to parent)
5. Destroys VSpace, CSpace, scheduler TCBs

**Returns:** `:ok`

**Note:** Can only terminate your own child realms (descendants).

---

### `realm-info`

Get information about a realm.

```clojure
(realm-info realm-id)
```

**Returns:**
```clojure
%{:name 'worker
  :status :running           ; :running :terminating :terminated
  :parent 3                  ; Parent realm-id (nil for root)
  :children #{8 9 12}        ; Child realm-ids
  :policy %{:cpu %{...} :memory %{...}}
  :resource-usage %{:cpu 0.15
                    :memory 1073741824}
  :process-count 42
  :scheduler-count 4}
```

---

### `parent-realm`

Get the parent realm's ID.

```clojure
(parent-realm)  ; → realm-id or nil (nil for root realm)
```

---

### `child-realms`

Get the set of child realm IDs.

```clojure
(child-realms)  ; → #{realm-id ...}
```

---

## Cross-Realm Process Creation

Spawn processes in descendant realms (children, grandchildren, etc.).

### `spawn-in`

Create a process in another realm.

```clojure
(spawn-in realm-id f)
(spawn-in realm-id f opts)
```

**Parameters:**
- `realm-id` — Target realm (must be a descendant)
- `f` — Zero-argument function to execute
- `opts` — Same options as `spawn`

**Returns:** `pid`

**Implementation:** Sends IPC to target realm's scheduler, which creates the process.

**Example:**
```clojure
(let [worker-realm (realm-create %{:name 'workers
                                    :policy %{:cpu %{:max 0.5}
                                              :memory %{:max (* 1 +GB+)}}})]
  ;; Spawn multiple workers in the child realm
  (dotimes [i 10]
    (spawn-in worker-realm (fn [] (worker-loop i)))))
```

---

### `spawn-link-in`

Create a linked process in another realm.

```clojure
(spawn-link-in realm-id f)
(spawn-link-in realm-id f opts)
```

**Returns:** `pid`

---

### `spawn-monitor-in`

Create a monitored process in another realm.

```clojure
(spawn-monitor-in realm-id f)
(spawn-monitor-in realm-id f opts)
```

**Returns:** `[pid monitor-ref]`

---

### Access Control

- You can `spawn-in` your **own realm** (equivalent to `spawn`, but slower due to IPC)
- You can `spawn-in` any **descendant realm** (children, grandchildren, etc.)
- You **cannot** `spawn-in` parent or sibling realms (no capability)

---

## Shared Memory

Zero-copy data sharing between realms via capability-granted memory mappings.

### `make-shared-region`

Create a shared memory region.

```clojure
(make-shared-region size name)
```

**Parameters:**
- `size` — Region size in bytes
- `name` — Symbol for identification

**Returns:** Region handle

**Example:**
```clojure
(def dataset (make-shared-region (* 1 +GB+) 'corpus))
```

---

### `share-region`

Grant access to a shared region to a child realm.

```clojure
(share-region region realm-id access)
```

**Parameters:**
- `region` — Region handle
- `realm-id` — Target realm (must be descendant)
- `access` — `:read-only` or `:read-write`

**Returns:** `:ok`

**Example:**
```clojure
(share-region dataset worker-realm :read-only)
```

---

### `unshare-region`

Revoke access to a shared region.

```clojure
(unshare-region region realm-id)
```

**Returns:** `:ok`

---

### `get-shared-region`

Get local mapping of a region shared with this realm.

```clojure
(get-shared-region name)
```

**Returns:** Region handle or `nil`

---

### `region-size`

Get the size of a region in bytes.

```clojure
(region-size region)  ; → integer
```

---

### Example: Parallel Data Processing

```clojure
;; Parent realm creates shared data
(def corpus (make-shared-region (* 1 +GB+) 'corpus))
(load-data-into-region! corpus "/data/corpus.bin")

;; Create worker realm and share data read-only
(let [workers (realm-create {:name 'workers
                              :policy {:cpu {:max 0.8}}})]
  (share-region corpus workers :read-only)

  ;; Spawn workers that process chunks
  (let [num-workers 8
        chunk-size (/ (region-size corpus) num-workers)]
    (dotimes [i num-workers]
      (spawn-in workers
        (fn []
          (let [data (get-shared-region 'corpus)]
            (process-chunk data (* i chunk-size) chunk-size)))))))
```

---

## Resource Management

Resources are managed at the realm level, not per-process.

### `request-memory`

Request additional memory from parent realm.

```clojure
(request-memory bytes)
```

**Returns:**
- `:ok` — Memory granted
- `[:error :over-max]` — Would exceed policy maximum
- `[:error :pool-exhausted]` — Parent's pool empty

---

### `return-memory`

Voluntarily return unused memory to parent's pool.

```clojure
(return-memory bytes)
```

**Returns:** `:ok`

---

### `memory-pressure-handler`

Register a callback for memory pressure signals from parent.

```clojure
(memory-pressure-handler f)
```

**Parameters:**
- `f` — Function receiving `:low`, `:critical`, or `:normal`

**Example:**
```clojure
(memory-pressure-handler
  (fn [level]
    (case level
      :critical (emergency-gc-all-processes!)
      :low (gc-idle-processes!)
      :normal nil)))
```

---

## Supervisors

Supervisors manage process lifecycles and implement restart strategies.

### `supervisor-start`

Start a supervisor process.

```clojure
(supervisor-start spec)
```

**Specification:**
```clojure
%{:strategy :one-for-one    ; or :one-for-all, :rest-for-one
  :max-restarts 3           ; Max restarts in time window
  :max-seconds 5            ; Time window for restart limit
  :children [%{:id 'worker-1
               :start (fn [] (worker-loop 1))
               :restart :permanent}
             %{:id 'worker-2
               :start (fn [] (worker-loop 2))
               :restart :permanent}]}
```

**Strategies:**
| Strategy | Behavior |
|----------|----------|
| `:one-for-one` | Only restart crashed child |
| `:one-for-all` | Restart all children if one crashes |
| `:rest-for-one` | Restart crashed child and all started after it |

**Restart Types:**
| Type | Behavior |
|------|----------|
| `:permanent` | Always restart |
| `:transient` | Restart only if abnormal exit |
| `:temporary` | Never restart |

**Note:** Supervisors manage processes within their realm. Realms themselves do not require supervision — a realm is a container that exists and holds processes. Only processes run and can crash; realms persist until explicitly terminated via `realm-terminate`. If all processes in a realm exit, the realm continues to exist (it can receive new `spawn-in` calls).

---

## API Reference

### Types

```
pid         := (pid realm-id local-id)  ; First-class type
realm-id    := integer                   ; Unique realm identifier
local-id    := integer                   ; Unique within realm
monitor-ref := opaque                    ; Monitor reference
region      := opaque                    ; Shared memory handle
```

### Process Creation (Current Realm)

```clojure
(spawn f)                    ; → pid
(spawn f opts)               ; → pid
(spawn-link f)               ; → pid
(spawn-link f opts)          ; → pid
(spawn-monitor f)            ; → [pid monitor-ref]
(spawn-monitor f opts)       ; → [pid monitor-ref]
```

### Process Creation (Other Realm)

```clojure
(spawn-in realm-id f)            ; → pid
(spawn-in realm-id f opts)       ; → pid
(spawn-link-in realm-id f)       ; → pid
(spawn-link-in realm-id f opts)  ; → pid
(spawn-monitor-in realm-id f)    ; → [pid monitor-ref]
(spawn-monitor-in realm-id f opts) ; → [pid monitor-ref]
```

### Process Identity

```clojure
(self)                       ; → pid
(self-realm)                 ; → realm-id
(pid-realm pid)              ; → realm-id
(pid-local pid)              ; → local-id
(pid? x)                     ; → boolean
(pid= p1 p2)                 ; → boolean
(alive? pid)                 ; → boolean (sync IPC if remote)
(process-info pid)           ; → map or nil
```

### Messaging

```clojure
(send pid msg)               ; → :ok (auto-routes local/remote)
(send! pid msg)              ; → :ok or :timeout
(send! pid msg timeout)      ; → :ok or :timeout
(send-named name msg)        ; → :ok or :not-found

(receive ...)                ; Blocking pattern match
(receive-nb ...)             ; Non-blocking
```

### Linking & Monitoring

```clojure
(link pid)                   ; → :ok (works cross-realm)
(unlink pid)                 ; → :ok
(trap-exit bool)             ; → :ok

(monitor pid)                ; → monitor-ref (works cross-realm)
(demonitor mref)             ; → :ok
```

### Lifecycle

```clojure
(exit reason)                ; Exit current process
(exit pid reason)            ; Send exit signal (works cross-realm)
```

### Registry

```clojure
(register name)              ; → :ok
(register name pid)          ; → :ok
(unregister name)            ; → :ok
(whereis name)               ; → pid or nil (hierarchical lookup)
(whereis-local name)         ; → pid or nil (local only)
```

### Realm Lifecycle

```clojure
(realm-create opts)          ; → realm-id
(realm-terminate realm-id)   ; → :ok
(realm-info realm-id)        ; → map
```

### Realm Hierarchy

```clojure
(self-realm)                 ; → realm-id
(parent-realm)               ; → realm-id or nil
(child-realms)               ; → #{realm-id}
```

### Shared Memory

```clojure
(make-shared-region size name)       ; → region
(share-region region realm-id access) ; → :ok
(unshare-region region realm-id)     ; → :ok
(get-shared-region name)             ; → region or nil
(region-size region)                 ; → integer
```

### Resources

```clojure
(request-memory bytes)       ; → :ok or [:error reason]
(return-memory bytes)        ; → :ok
(memory-pressure-handler f)  ; → :ok
```

### Supervisors

```clojure
(supervisor-start spec)      ; → pid
```

### Notifications

Notifications provide lightweight signaling for event-driven patterns, particularly useful for IRQ handling and inter-process coordination.

```clojure
(make-notification)          ; → notification
```

**Returns:** A notification object (wraps seL4 notification capability)

**Usage:**
- Create a notification to receive signals from IRQ handlers or other processes
- Bind to IRQ via `irq-register!` (see `lona.io`)
- Wait/signal/poll using `lona.kernel` primitives: `wait!`, `signal!`, `poll!`

**Example:**
```clojure
(let [ntfn (make-notification)]
  ;; Register for IRQ (see lona.io)
  (io/irq-register! irq-num ntfn)

  ;; Wait for signal (from lona.kernel)
  (k/wait! ntfn)  ; blocks until signaled, returns badge

  ;; Handle interrupt...
  (io/irq-ack! irq-handler))
```

**Note:** Notifications are userspace constructs backed by seL4 notification objects. The underlying operations (`signal!`, `wait!`, `poll!`) are provided by `lona.kernel`. This namespace provides `make-notification` for convenient allocation.

---

## Example: Multi-Realm Worker Pool

```clojure
(ns myapp.distributed-pool
  (:require [lona.process :as proc]))

(defn worker-loop [id]
  (receive
    [:task data from]
      (do
        (let [result (process-task data)]
          (proc/send from [:result id result]))
        (worker-loop id))
    [:shutdown]
      (proc/exit :normal)))

(defn manager-loop [workers next-worker]
  (receive
    [:submit task from]
      (do
        (proc/send (nth workers next-worker) [:task task from])
        (manager-loop workers (mod (inc next-worker) (count workers))))
    [:shutdown]
      (do
        (doseq [w workers]
          (proc/send w [:shutdown]))
        (proc/exit :normal))))

(defn start-distributed-pool [num-workers]
  ;; Create isolated worker realm with resource limits
  (let [worker-realm (proc/realm-create
                       %{:name 'worker-pool
                         :policy %{:cpu %{:min 0.1 :max 0.5}
                                   :memory %{:min (* 100 +MB+)
                                             :max (* 500 +MB+)}}})]

    ;; Spawn workers in the isolated realm
    (let [workers (vec (for [i (range num-workers)]
                         (proc/spawn-link-in worker-realm
                           (fn [] (worker-loop i)))))]

      ;; Manager stays in parent realm
      (proc/spawn (fn [] (manager-loop workers 0))))))
```
