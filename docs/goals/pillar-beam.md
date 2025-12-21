# Pillar II: BEAM/OTP — The Engine

> *"Let It Crash"*

## Why BEAM/OTP?

The BEAM virtual machine and OTP framework power some of the most reliable systems ever built. Ericsson's telephone switches achieve "nine nines" availability (99.9999999% uptime—about 31 milliseconds of downtime per year). WhatsApp handles millions of concurrent connections per server. Discord routes billions of messages.

This reliability comes not from preventing failures, but from embracing them:

1. **Processes are cheap**: Create millions of them. Each handles one thing.
2. **Processes are isolated**: A crash in one doesn't corrupt others.
3. **Supervision hierarchies**: When something crashes, restart it automatically.
4. **Message passing**: No shared mutable state to corrupt.

Lona adopts this model completely. Every piece of running code executes within a Process. All communication happens through messages. Failures are expected, contained, and recovered automatically.

---

## Philosophy: Let It Crash

Traditional error handling tries to anticipate every failure and handle it locally:

```
// Defensive programming
result = doSomething()
if (result.error) {
    // Try recovery A
    // If that fails, try recovery B
    // If that fails, try recovery C
    // If that fails, log and... ???
}
```

This approach leads to complex, nested error handling that's hard to reason about and often incomplete.

The "Let It Crash" philosophy inverts this:

1. **Write the happy path**: Code assumes everything works
2. **Crash on failure**: When something unexpected happens, the process dies
3. **Supervisor restarts**: A separate supervisor process detects the crash and restarts
4. **Fresh state**: The restarted process begins with known-good initial state

```clojure
;; No defensive error handling needed
(defn handle-request [request]
  (let [user (get-user! (:user-id request))     ; crashes if not found
        data (fetch-data! (:data-id request))   ; crashes on timeout
        result (process user data)]             ; crashes on invalid data
    {:ok result}))

;; Supervisor handles all failures uniformly
(def-supervisor request-handler-sup
  :strategy :one-for-one
  :children [{:id :handler :start #(spawn handle-request-loop)}])
```

This is not about ignoring errors. It's about:
- Separating error detection from error recovery
- Centralizing recovery logic in supervisors
- Ensuring processes always start in known-good state
- Building systems that self-heal

---

## What BEAM/OTP Forces in Lona

### 1. Lightweight Processes

Lona Processes are not OS threads. They are:

| Property | Value |
|----------|-------|
| **Overhead** | Hundreds of bytes (vs. megabytes for OS threads) |
| **Scalability** | Millions concurrent (vs. thousands for threads) |
| **Scheduling** | Preemptive via reduction counting |
| **Isolation** | Separate heaps, independent GC |

Creating a process is cheap. Destroying a process is cheap. This changes how you design systems—use a process for each logical unit of work, not each CPU core.

### 2. Message Passing Only

Processes communicate exclusively through message passing:

```clojure
;; Send a message
(send pid {:request :get-user :id 42})

;; Receive with pattern matching
(receive
  {:response :ok :user user}
    (process-user user)
  {:response :error :reason reason}
    (handle-error reason)
  (after 5000
    (handle-timeout)))
```

There is no shared mutable state between processes. No locks, no mutexes, no races. Messages are copied (or, with Clojure's immutable data, safely shared).

### 3. Fault Containment

A crashing process affects only itself:
- Its heap is released
- Its mailbox is discarded
- Linked processes are notified
- Other processes continue running

Because each process has its own heap and is garbage collected independently, one process's crash cannot corrupt another's memory (within managed Lonala code).

### 4. Supervision Trees

Processes are organized into hierarchies where parent supervisors manage child workers:

```
           [Application Supervisor]
                    │
       ┌───────────┼───────────┐
       │           │           │
 [Pool Sup]   [Cache Sup]  [Queue Sup]
       │           │           │
   ┌───┴───┐       │       ┌───┴───┐
   │   │   │    [Cache]    │       │
[W1] [W2] [W3]          [Writer] [Reader]
```

When a worker crashes, its supervisor decides what to do based on strategy:

| Strategy | Behavior |
|----------|----------|
| `:one-for-one` | Restart only the failed child |
| `:one-for-all` | Restart all children if one fails |
| `:rest-for-one` | Restart failed child and all started after it |

---

## The Process Abstraction

A Process is the fundamental unit of execution in Lona:

### Process Properties

| Property | Description |
|----------|-------------|
| **Execution unit** | Runs code, maintains call stack |
| **Mailbox** | Queue of incoming messages |
| **Heap** | Private memory, independently GC'd |
| **Links** | Bidirectional failure notification |
| **Monitors** | Unidirectional observation |

### Process Lifecycle

```
spawn → running ←→ waiting → exit
              ↓
         suspended (preemption)
```

Processes are:
- **Created** via `spawn`
- **Running** when executing code
- **Waiting** when blocked in `receive`
- **Suspended** when preempted by scheduler
- **Exited** on completion or crash

### Links and Monitors

**Links** create bidirectional failure coupling:

```clojure
(link pid)  ; if either process dies, both are notified

;; Or atomically at spawn
(spawn-link worker-fn args)
```

**Monitors** create unidirectional observation:

```clojure
(def ref (monitor pid))  ; we're notified if pid dies

;; Later
(demonitor ref)
```

| Feature | Link | Monitor |
|---------|------|---------|
| Direction | Bidirectional | Unidirectional |
| Exit propagation | Yes (by default) | No |
| Use case | Tightly coupled processes | Observing without coupling |

---

## Supervision

Supervision is the mechanism that makes "let it crash" work. Supervisors are processes that:
1. Start child processes
2. Monitor them for crashes
3. Restart them according to strategy
4. Escalate if restarts fail

### Defining a Supervisor

```clojure
(def-supervisor database-supervisor
  :strategy :one-for-one
  :max-restarts 5
  :max-seconds 60
  :children
  [{:id :connection-pool
    :start #(spawn pool/start [db-config])
    :restart :permanent}
   {:id :query-cache
    :start #(spawn cache/start [cache-config])
    :restart :permanent}
   {:id :stats-collector
    :start #(spawn stats/start [])
    :restart :transient}])
```

### Restart Types

| Type | Behavior |
|------|----------|
| `:permanent` | Always restart when terminated |
| `:transient` | Restart only on abnormal termination |
| `:temporary` | Never restart |

### Restart Intensity

Supervisors track restart frequency. If a child restarts too many times too quickly (e.g., 5 times in 60 seconds), the supervisor itself terminates—escalating to its parent.

This prevents infinite restart loops and propagates the failure upward until something can actually handle it.

---

## Cross-Domain Supervision

A unique Lona feature: supervisors can manage processes across Domain boundaries.

```
Domain: services
├── Process: tcp-supervisor
│   │
│   │   (supervises across domain boundary)
│   │
│   ├── Domain: connection-handler-1
│   │   └── Process: handler
│   │
│   └── Domain: connection-handler-2
│       └── Process: handler
```

From the supervisor's perspective:
- Child processes are managed uniformly regardless of Domain
- Crashes are reported through the same mechanism
- Restart strategies apply the same way

The Domain boundary affects **security isolation**, not **supervision semantics**.

---

## Concurrency Model

### Scheduling

Lona uses preemptive scheduling based on reduction counting:

- Each process gets a "reduction budget" (roughly, number of function calls)
- When budget exhausted, process yields
- Scheduler picks next runnable process
- No process can monopolize the CPU

### Per-Process Garbage Collection

Each process has its own heap:
- GC runs per-process, not globally
- One process's GC pause doesn't affect others
- Memory is reclaimed immediately when process exits

This is critical for low-latency systems. A process handling user requests isn't paused when a background process triggers GC.

---

## Implications for Lona Design

BEAM/OTP constraints shape these Lona design decisions:

| Decision | Driven By |
|----------|-----------|
| Processes as fundamental unit | Cheap, isolated, message-passing |
| No shared mutable state | Message passing only |
| Supervision trees | Systematic fault recovery |
| Per-process GC | Isolation, latency |
| Process-local atoms | No STM needed |

---

## Summary

BEAM/OTP provides Lona with:

| Guarantee | Mechanism |
|-----------|-----------|
| **Massive concurrency** | Lightweight processes |
| **Fault isolation** | Separate heaps |
| **Self-healing** | Supervision trees |
| **Deadlock freedom** | No shared state |
| **Low latency** | Per-process GC |

**The Bottom Line**: In Lona, you don't write defensive error handling. You write the happy path, let processes crash on unexpected failures, and let supervisors restart them. The system self-heals.

---

## Further Reading

- [Core Concepts: Process](core-concepts.md#process)
- [Core Concepts: Supervisor](core-concepts.md#supervisor)
- [System Design: Concurrency Model](system-design.md#concurrency-model)
- [Erlang: A History](https://www.erlang.org/about)
- [Learn You Some Erlang: Supervisors](https://learnyousomeerlang.com/supervisors)
