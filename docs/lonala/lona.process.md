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

**Options:**

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `:min-heap-size` | integer | 4096 | Initial heap bytes |
| `:max-heap-size` | integer | nil | Maximum heap (nil = unlimited) |
| `:priority` | 0-255 | 100 | Scheduling priority |
| `:name` | symbol | nil | Register with name |

### `spawn-link`

Create linked process. Bidirectional crash notification.

```clojure
(spawn-link f)       ; → pid
(spawn-link f opts)  ; → pid
```

### `spawn-monitor`

Create monitored process. Unidirectional notification.

```clojure
(spawn-monitor f)       ; → [pid monitor-ref]
(spawn-monitor f opts)  ; → [pid monitor-ref]
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

### `spawn-link-in`

Create linked process in descendant realm.

```clojure
(spawn-link-in realm-id f)       ; → pid
(spawn-link-in realm-id f opts)  ; → pid
```

### `spawn-monitor-in`

Create monitored process in descendant realm.

```clojure
(spawn-monitor-in realm-id f)       ; → [pid monitor-ref]
(spawn-monitor-in realm-id f opts)  ; → [pid monitor-ref]
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
(self)  ; → pid
```

### `self-realm`

Current realm ID.

```clojure
(self-realm)  ; → realm-id
```

### `alive?`

Check if process exists.

```clojure
(alive? pid)  ; → boolean
```

### `process-info`

Get process information.

```clojure
(process-info pid)  ; → map or nil
```

Returns map with `:status`, `:heap-size`, `:mailbox-len`, `:priority`, `:links`, `:monitors`.

---

## Messaging

### `send`

Send message asynchronously.

```clojure
(send pid message)  ; → :ok
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

### `send-named`

Send to registered name.

```clojure
(send-named name message)  ; → :ok or :not-found
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

### `unlink`

Remove link.

```clojure
(unlink pid)  ; → :ok
```

### `trap-exit`

Enable/disable exit signal trapping.

```clojure
(trap-exit true)   ; Receive [:EXIT pid reason] as messages
(trap-exit false)  ; Propagate crashes (default)
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

### `demonitor`

Stop monitoring.

```clojure
(demonitor monitor-ref)  ; → :ok
```

### Monitor Messages

When monitored process exits:

```clojure
[:DOWN monitor-ref pid reason]
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

### `unregister`

Remove registration.

```clojure
(unregister name)  ; → :ok
```

### `whereis`

Hierarchical name lookup.

```clojure
(whereis name)  ; → pid or nil
```

Searches: current realm → parent → grandparent → root.

### `whereis-local`

Local realm lookup only.

```clojure
(whereis-local name)  ; → pid or nil
```

---

## Realm Management

### `realm-create`

Create child realm.

```clojure
(realm-create opts)  ; → realm-id
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

### `realm-info`

Get realm information.

```clojure
(realm-info realm-id)  ; → map
```

Returns: `:name`, `:status`, `:parent`, `:children`, `:policy`, `:resource-usage`, `:process-count`.

### `parent-realm`

Get parent realm ID.

```clojure
(parent-realm)  ; → realm-id or nil
```

### `child-realms`

Get child realm IDs.

```clojure
(child-realms)  ; → #{realm-id ...}
```

---

## Shared Memory

### `make-shared-region`

Create shared memory region.

```clojure
(make-shared-region size name)  ; → region
```

### `share-region`

Grant region access to child realm.

```clojure
(share-region region realm-id access)  ; → :ok
```

Access: `:read-only` or `:read-write`

### `unshare-region`

Revoke region access.

```clojure
(unshare-region region realm-id)  ; → :ok
```

### `get-shared-region`

Get region shared with this realm.

```clojure
(get-shared-region name)  ; → region or nil
```

### `region-size`

Get region size.

```clojure
(region-size region)  ; → integer
```

### `region-name`

Get region name.

```clojure
(region-name region)  ; → symbol
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

### `return-memory`

Return memory to parent realm.

```clojure
(return-memory bytes)  ; → :ok
```

The VM runtime releases Untyped capabilities back to the parent.

### `memory-pressure-handler`

Register pressure callback.

```clojure
(memory-pressure-handler f)  ; → :ok
```

Callback receives: `:low`, `:critical`, or `:normal`.

---

## Notifications

### `make-notification`

Create notification object.

```clojure
(make-notification)  ; → notification
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
