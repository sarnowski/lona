# lona.process

Process and realm management intrinsics. These provide BEAM-style lightweight processes with message passing.

---

## Process Creation

### `spawn`

Create process in current realm.

```clojure
(spawn f)       ; → pid
(spawn f opts)  ; → pid
```

```clojure
;; @todo
(def p (spawn (fn* [] :ok)))
(pid? p)  ; => true
```

**Options:**

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `:min-heap-size` | integer | 4096 | Initial heap bytes |
| `:max-heap-size` | integer | nil | Maximum heap (nil = unlimited) |
| `:priority` | 0-255 | 100 | Scheduling priority |
| `:name` | symbol | nil | Register with name |

```clojure
;; @todo
;; Spawn with priority option
(def p (spawn (fn* [] (sleep 1000)) %{:priority 50}))
(pid? p)  ; => true
(= (:priority (process-info p)) 50)  ; => true

;; Spawn with name auto-registers
(def p2 (spawn (fn* [] (sleep 1000)) %{:name 'named-process}))
(= (whereis 'named-process) p2)  ; => true
```

### `spawn-link`

Create linked process. Bidirectional crash notification.

```clojure
(spawn-link f)       ; → pid
(spawn-link f opts)  ; → pid
```

```clojure
;; @todo
(def p (spawn-link (fn* [] :ok)))
(pid? p)  ; => true
```

### `spawn-monitor`

Create monitored process. Unidirectional notification.

```clojure
(spawn-monitor f)       ; → [pid monitor-ref]
(spawn-monitor f opts)  ; → [pid monitor-ref]
```

```clojure
;; @todo
(def result (spawn-monitor (fn* [] :ok)))
(tuple? result)        ; => true
(pid? (first result))  ; => true
(ref? (nth result 1))  ; => true
```

---

## Cross-Realm Process Creation

### `spawn-in`

Create process in descendant realm.

```clojure
(spawn-in realm-id f)                     ; → pid
(spawn-in realm-id f arg1 arg2 ...)       ; → pid, with arguments
(spawn-in realm-id f arg1 arg2 ... opts)  ; → pid, with arguments and options
```

Arguments are passed to `f` when the process starts. If the last argument is a map
with spawn options (`:min-heap-size`, `:priority`), it is used as options, not passed
to `f`.

```clojure
;; @todo
(def child-realm (realm-create %{:name 'spawn-in-test}))
(def p (spawn-in child-realm my-worker))
(pid? p)  ; => true
(realm-terminate child-realm)
```

### `spawn-link-in`

Create linked process in descendant realm.

```clojure
(spawn-link-in realm-id f)       ; → pid
(spawn-link-in realm-id f opts)  ; → pid
```

```clojure
;; @todo
(def child-realm (realm-create %{:name 'spawn-link-test}))
(def p (spawn-link-in child-realm my-worker))
(pid? p)  ; => true
(realm-terminate child-realm)
```

### `spawn-monitor-in`

Create monitored process in descendant realm.

```clojure
(spawn-monitor-in realm-id f)       ; → [pid monitor-ref]
(spawn-monitor-in realm-id f opts)  ; → [pid monitor-ref]
```

```clojure
;; @todo
(def child-realm (realm-create %{:name 'spawn-monitor-test}))
(def result (spawn-monitor-in child-realm my-worker))
(tuple? result)        ; => true
(pid? (first result))  ; => true
(ref? (nth result 1))  ; => true
(realm-terminate child-realm)
```

### Cross-Realm Function Execution

**Function must be a var reference:**

The function `f` must be defined via `def` and referenced by its var. Anonymous
functions and closures cannot be transferred across realm boundaries.

```clojure
;; Works: var reference with arguments
(spawn-in child-realm my-worker arg1 arg2)

;; Error: anonymous function
(spawn-in child-realm (fn [] (do-work)))

;; Error: closure with captured bindings
(let [x 42]
  (spawn-in child-realm (fn [] (use x))))

;; Instead, pass data as arguments
(let [x 42]
  (spawn-in child-realm my-worker x))  ; x is deep-copied
```

**Code sharing:**

Parent realm's code pages are mapped read-only into child realms during realm
creation. The spawned function must reside in these shared pages.

**Arguments:**

Arguments to the spawned function are deep-copied across the realm boundary,
using the same serialization as inter-realm messages.

**Scope:**

`spawn-in` only works with **descendant realms** (child, grandchild, etc.):
- You cannot spawn into parent realms
- You cannot spawn into sibling or unrelated realms
- Attempting to spawn into a non-descendant returns `[:error :not-descendant]`

---

## Process Identity

### `self`

Current process PID.

```clojure
;; @todo
(pid? (self))              ; => true
```

```clojure
(= (self) (self))          ; => true
```

```clojure
;; @todo
(pid-realm (self))         ; => (self-realm)
```

### `self-realm`

Current realm ID.

```clojure
;; @todo
(realm-id? (self-realm))   ; => true
```

### `alive?`

Check if process exists.

```clojure
;; @todo
(alive? (self))            ; => true

;; Dead process returns false
(def p (spawn (fn* [] :done)))
(sleep 10)
(alive? p)                 ; => false
```

### `process-info`

Get process information.

```clojure
(def info (process-info))
(map? info)                   ; => true
(contains? info :status)      ; => true
(contains? info :heap-size)   ; => true
```

```clojure
;; @todo
(def info (process-info (self)))
(contains? info :mailbox-len) ; => true
(contains? info :priority)    ; => true
(contains? info :links)       ; => true
(contains? info :monitors)    ; => true
```

Returns map with:

| Key | Type | Description |
|-----|------|-------------|
| `:status` | keyword | Process status (`:running`, `:waiting`, etc.) |
| `:heap-size` | integer | Current young heap size in bytes |
| `:heap-used` | integer | Young heap bytes in use |
| `:old-heap-size` | integer | Current old heap size in bytes |
| `:old-heap-used` | integer | Old heap bytes in use |
| `:stack-size` | integer | Current stack size in bytes |
| `:minor-gc-count` | integer | Number of minor GCs performed |
| `:major-gc-count` | integer | Number of major GCs performed |
| `:total-reclaimed` | integer | Total bytes reclaimed by GC |
| `:mailbox-len` | integer | Messages in mailbox |
| `:priority` | integer | Scheduling priority (0-255) |
| `:links` | set | PIDs of linked processes |
| `:monitors` | set | Monitor references |
| `:reductions` | integer | Total reductions executed |

---

## Messaging

### `send`

Send message asynchronously.

```clojure
(send pid message)  ; → :ok
```

```clojure
;; @todo
(send (self) :hello)  ; => :ok
(send (self) [:ok 42])  ; => :ok
```

Raises error if `pid` is not a PID:

```clojure
;; @todo
(send 42 :msg)        ; => ERROR: bad argument
(send nil :msg)       ; => ERROR: bad argument
(send "hello" :msg)   ; => ERROR: bad argument
```

Sending to a dead PID is silently ignored (BEAM semantics):

```clojure
;; @todo
(let [p (spawn (fn* [] :done))]
  (send p :after-death))  ; => :ok (no error, message dropped)
```

- Intra-realm: deep copy, ~100-500 ns
- Inter-realm: seL4 IPC, ~1-10 µs
- Order preserved between same sender-receiver pair

### `send-sync`

Send message and wait for acknowledgment.

```clojure
(send-sync pid message)              ; → :ok or :timeout
(send-sync pid message timeout-ms)   ; → :ok or :timeout
```

```clojure
;; @todo
;; send-sync with zero timeout returns :timeout if not acknowledged
(send-sync (self) :msg 0)  ; => :timeout
```

### `send-named`

Send to registered name.

```clojure
(send-named name message)  ; → :ok or :not-found
```

```clojure
;; @todo
(send-named 'nonexistent :msg)  ; => :not-found

;; Success case: send to registered name
(register 'send-named-test)
(send-named 'send-named-test :hello)  ; => :ok
(receive msg msg :after 100 :timeout)  ; => :hello
(unregister 'send-named-test)
```

### `receive`

Pattern-matched message reception with optional timeout. This is a VM-supported
construct (not a simple macro) for efficient single-pass pattern matching.

```clojure
(receive
  pattern1 body1
  pattern2 when guard body2
  :after timeout-ms timeout-body)
```

**Semantics:**

1. Scan mailbox messages in order
2. For each message, try full pattern matching against all clauses (in order)
3. First pattern that matches: remove message, bind variables, execute body
4. If no pattern matches this message: skip it, continue to next message
5. If no messages match: block waiting for new messages
6. If `:after` specified and timeout expires: execute timeout-body

**Key properties:**

- **Single-pass matching** — full pattern matching during scan, no double evaluation
- **Selective receive** — non-matching messages remain in mailbox (Erlang semantics)
- **Guards** — `when` clauses evaluated only if pattern structurally matches

```clojure
(receive
  %{:type :request :id id :payload p}
    (handle-request id p)

  %{:type :shutdown}
    (cleanup-and-exit)

  [:EXIT pid reason] when (not= reason :normal)
    (handle-crash pid reason)

  :after 5000
    (handle-timeout))
```

Timeout returns immediately if mailbox empty:

```clojure
(receive :after 0 :timeout)  ; => :timeout
```

Receive with pattern matching:

```clojure
;; @todo
(send (self) [:ok 42])
(receive [:ok val] val :after 100 :timeout)  ; => 42
```

Selective receive (non-matching messages remain):

```clojure
;; @todo
(send (self) :a)
(send (self) [:b 1])
(receive [:b x] x :after 100 :timeout)  ; => 1
(receive :a :got-a :after 100 :timeout)  ; => :got-a
```

Message ordering preserved:

```clojure
;; @todo
(send (self) 1)
(send (self) 2)
(send (self) 3)
(receive x x :after 100 :timeout)  ; => 1
(receive x x :after 100 :timeout)  ; => 2
(receive x x :after 100 :timeout)  ; => 3
```

### Message Value Types

**Intra-realm messages (same realm):**

All Lonala values can be sent, including:
- Primitives, collections, binaries
- Functions and closures
- Capability wrappers (`notification`, `endpoint`, etc.)

Values are deep-copied to the receiver's heap.

**Inter-realm messages (cross-realm):**

All serializable values can be sent, including:
- Primitives: nil, booleans, numbers, strings, symbols, keywords
- Collections: lists, tuples, vectors, maps, sets (with serializable contents)
- Binaries (reference-counted, not copied)
- **Capability wrappers** — the kernel transfers the underlying capability

```clojure
;; Granting a notification to another realm via message
(send worker-in-child-realm %{:task data :done-signal my-notif})
;; Receiver gets a copy of the notification capability
```

**Cannot be sent inter-realm:**
- Functions and closures (use var references via `spawn-in` instead)
- Bytebufs (mutable, not shareable)
- Vars (send the var's value, not the var itself)

**Enforcement:**

- Intra-realm: no enforcement (same VSpace, direct copy)
- Inter-realm: VM serializes message; kernel handles capability transfer via seL4 IPC

---

## Linking

### `link`

Create bidirectional link.

```clojure
(link pid)  ; → :ok
```

```clojure
;; @todo
(def p (spawn (fn* [] (sleep 1000))))
(link p)  ; => :ok
```

### `unlink`

Remove link.

```clojure
(unlink pid)  ; → :ok
```

```clojure
;; @todo
(def p (spawn (fn* [] (sleep 1000))))
(link p)
(unlink p)  ; => :ok
```

### `trap-exit`

Enable/disable exit signal trapping.

```clojure
(trap-exit true)   ; Receive [:EXIT pid reason] as messages
(trap-exit false)  ; Propagate crashes (default)
```

```clojure
;; @todo
(trap-exit true)   ; => :ok
(trap-exit false)  ; => :ok
```

With trap-exit, linked process exits become messages:

```clojure
;; @todo
(trap-exit true)
(def p (spawn-link (fn* [] (exit :test-exit))))
(receive
  [:EXIT pid reason] [pid reason]
  :after 1000 :timeout)  ; => [p :test-exit]
```

### Exit Signals

When linked process exits, linked processes receive:

```clojure
[:EXIT pid reason]
```

Reasons: `:normal`, `:killed`, `[:error type]`, or custom term.

---

## Monitoring

### `monitor`

Start monitoring process.

```clojure
(monitor pid)  ; → monitor-ref
```

```clojure
;; @todo
(def p (spawn (fn* [] (sleep 1000))))
(def mref (monitor p))
(ref? mref)  ; => true
```

### `demonitor`

Stop monitoring.

```clojure
(demonitor monitor-ref)  ; → :ok
```

```clojure
;; @todo
(def p (spawn (fn* [] (sleep 1000))))
(def mref (monitor p))
(demonitor mref)  ; => :ok
```

### Monitor Messages

When monitored process exits:

```clojure
[:DOWN monitor-ref pid reason]
```

```clojure
;; @todo
(def p (spawn (fn* [] :done)))
(def mref (monitor p))
(receive [:DOWN mref _ reason] reason :after 1000 :timeout)  ; => :normal
```

---

## Lifecycle

### `exit`

Terminate process or send exit signal.

```clojure
(exit reason)       ; Exit current process with reason
(exit pid reason)   ; Send exit signal to another process
```

### Exit Reasons

Exit reasons are ordinary Lonala values. Common patterns:

| Reason | Meaning | Cascades to links? |
|--------|---------|-------------------|
| `:normal` | Clean exit | No |
| `:shutdown` | Clean shutdown | Yes (but expected) |
| `:killed` | Process was force-killed | Yes |
| `[:error type info]` | Error with details | Yes |
| Other term | Application-defined | Yes |

### Exit Signals

When sending exit signal to another process:

| Signal Sent | What Receiver Sees | Trappable? |
|-------------|-------------------|------------|
| `:normal` | `:normal` | Yes |
| `:kill` | `:killed` | **No** (always terminates) |
| `:shutdown` | `:shutdown` | Yes |
| Other | Same value | Yes |

**Note:** Sending `:kill` is transformed to `:killed` at the receiver.

### Link Cascade Behavior

When process A (linked to B) exits with reason R:

- **R is `:normal`:** B is NOT affected (clean exit doesn't cascade)
- **R is anything else:** B receives exit signal with reason R
  - If B is not trapping exits: B exits with same reason
  - If B is trapping exits: B receives `[:EXIT pid-of-A R]` message

### Runtime Errors

Runtime errors (match failure, bad arguments, etc.) cause the process to exit
with an error-shaped reason:

| Error | Exit Reason |
|-------|-------------|
| Match failure | `[:error :badmatch %{:value v}]` |
| Bad argument | `[:error :badarg %{:fn f :args args}]` |
| Undefined var | `[:error :undef %{:var v}]` |
| Out of memory | `[:error :oom]` |

These are not special types — they're just values that supervisors can pattern match.

---

## Registry

### `register`

Register process with name.

```clojure
(register name)       ; Register current process
(register name pid)   ; Register specific process
```

```clojure
;; @todo
(register 'my-test-process)
(pid? (whereis 'my-test-process))  ; => true
(= (whereis 'my-test-process) (self))  ; => true
```

### `unregister`

Remove registration.

```clojure
(unregister name)  ; → :ok
```

```clojure
;; @todo
(register 'to-unregister)
(unregister 'to-unregister)  ; => :ok
(whereis 'to-unregister)     ; => nil
```

### `whereis`

Hierarchical name lookup.

```clojure
(whereis name)  ; → pid or nil
```

```clojure
;; @todo
(whereis 'nonexistent)  ; => nil
```

Searches: current realm → parent → grandparent → root.

```clojure
;; @todo
;; whereis finds registered process
(def p (spawn (fn* [] (sleep 1000))))
(register 'whereis-test p)
(= (whereis 'whereis-test) p)  ; => true
(unregister 'whereis-test)
```

### `whereis-local`

Local realm lookup only.

```clojure
(whereis-local name)  ; → pid or nil
```

```clojure
;; @todo
(whereis-local 'nonexistent)  ; => nil
```

---

## Realm Management

### `realm-create`

Create child realm.

```clojure
(realm-create opts)  ; → realm-id
```

```clojure
;; @todo
(def r (realm-create %{:name 'test-child}))
(realm-id? r)  ; => true
```

**Options:**

| Key | Required | Description |
|-----|----------|-------------|
| `:name` | yes | Realm name |
| `:policy` | no | Resource policy |
| `:schedulers` | no | `:auto` or integer |
| `:shared` | no | Regions to share |

**Shared regions format:**

```clojure
%{:region region-handle :access :read-only}  ; or :read-write
```

**Policy:**

```clojure
%{:cpu %{:min 0.1 :max 0.3}
  :memory %{:min size :max size}}
```

### `realm-terminate`

Terminate child realm.

```clojure
(realm-terminate realm-id)  ; → :ok
```

Terminates all processes and child realms, reclaims resources.

```clojure
;; @todo
(def r (realm-create %{:name 'to-terminate}))
(realm-terminate r)  ; => :ok
```

### `realm-info`

Get realm information.

```clojure
(realm-info realm-id)  ; → map
```

Returns: `:name`, `:status`, `:parent`, `:children`, `:policy`, `:resource-usage`, `:process-count`.

```clojure
;; @todo
(def info (realm-info (self-realm)))
(map? info)                  ; => true
(contains? info :name)       ; => true
(contains? info :status)     ; => true
(contains? info :process-count) ; => true
```

### `parent-realm`

Get parent realm ID.

```clojure
(parent-realm)  ; → realm-id or nil
```

```clojure
;; @todo
;; parent-realm returns realm-id or nil (if root)
(or (nil? (parent-realm)) (realm-id? (parent-realm)))  ; => true
```

### `child-realms`

Get child realm IDs.

```clojure
(child-realms)  ; → #{realm-id ...}
```

```clojure
;; @todo
(set? (child-realms))  ; => true
```

---

## Shared Memory

### `make-shared-region`

Create shared memory region.

```clojure
(make-shared-region size name)  ; → region
```

```clojure
;; @todo
(def r (make-shared-region 4096 'test-region))
(region? r)  ; => true
```

### `share-region`

Grant region access to child realm.

```clojure
(share-region region realm-id access)  ; → :ok
```

Access: `:read-only` or `:read-write`

```clojure
;; @todo
(def r (make-shared-region 4096 'share-test))
(def child (realm-create %{:name 'share-child}))
(share-region r child :read-only)  ; => :ok
(realm-terminate child)
```

### `unshare-region`

Revoke region access.

```clojure
(unshare-region region realm-id)  ; → :ok
```

```clojure
;; @todo
(def r (make-shared-region 4096 'unshare-test))
(def child (realm-create %{:name 'unshare-child}))
(share-region r child :read-write)
(unshare-region r child)  ; => :ok
(realm-terminate child)
```

### `get-shared-region`

Get region shared with this realm.

```clojure
(get-shared-region name)  ; → region or nil
```

```clojure
;; @todo
(get-shared-region 'nonexistent)  ; => nil
```

### `region-size`

Get region size.

```clojure
(region-size region)  ; → integer
```

```clojure
;; @todo
(def r (make-shared-region 4096 'size-test))
(region-size r)  ; => 4096
```

### `region-name`

Get region name.

```clojure
(region-name region)  ; → symbol
```

```clojure
;; @todo
(def r (make-shared-region 4096 'name-test))
(region-name r)  ; => name-test
```

---

## Resources

### `request-memory`

Request additional memory from parent realm.

```clojure
(request-memory bytes)  ; → :ok or [:error reason]
```

On success, the parent grants Untyped capabilities which are automatically installed
in the realm's CSpace. The VM runtime's allocator is notified and can use these for
subsequent allocations. Application code does not interact with the capabilities
directly — use `lona.kernel` for low-level capability manipulation.

```clojure
;; @todo
(def result (request-memory 4096))
(or (= result :ok) (tuple? result))  ; => true
```

### `return-memory`

Return memory to parent realm.

```clojure
(return-memory bytes)  ; → :ok
```

The VM runtime releases Untyped capabilities back to the parent.

```clojure
;; @todo
(return-memory 4096)  ; => :ok
```

### `memory-pressure-handler`

Register pressure callback.

```clojure
(memory-pressure-handler f)  ; → :ok
```

Callback receives: `:low`, `:critical`, or `:normal`.

```clojure
;; @todo
(memory-pressure-handler (fn* [level] level))  ; => :ok
```

---

## Notifications

### `make-notification`

Create notification object.

```clojure
(make-notification)  ; → notification
```

```clojure
;; @todo
(def n (make-notification))
(notification? n)  ; => true
```

Used with `lona.kernel` signal/wait/poll operations.

---

## Appendix: Expected Derived Functions

The following are **not intrinsics** and should be implemented in Lonala:

**Supervisor:**

```clojure
(supervisor-start spec)
```

Implements restart strategies using `spawn-link`, `trap-exit`, `receive`. Semantics
follow Erlang/OTP supervisors: supports one-for-one, one-for-all, and rest-for-one
restart strategies with configurable restart intensity limits.

**Other:**

- `receive-nb` (non-blocking receive)
- `call` (synchronous request-response with `make-ref`)
