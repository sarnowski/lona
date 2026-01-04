# Process Model

This document covers Lonala processes: lightweight execution units within realms, their scheduling, message passing, and garbage collection.

## Process Characteristics

Lonala processes are modeled after BEAM/Erlang processes:

| Property | Description |
|----------|-------------|
| **Lightweight** | ~1-10 µs to spawn, minimal memory overhead |
| **Isolated heap** | Each process has its own heap, no shared mutable state |
| **Mailbox** | Each process has an incoming message queue |
| **Fault isolation** | Process crash doesn't affect other processes |
| **Millions per realm** | A single realm can host millions of processes |

### Process Structure

```
PROCESS (Pure Userspace Construct)
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  Identity:                                                          │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  pid: ProcessId        - Unique within realm                │    │
│  │  parent: ProcessId     - Who spawned this process           │    │
│  │  links: Set<ProcessId> - Bidirectional crash notification   │    │
│  │  monitors: Set<MonitorRef> - Unidirectional monitoring      │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                     │
│  Execution State:                                                   │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  status: Running | Waiting | Exited                         │    │
│  │  reductions: u32       - Instructions until yield           │    │
│  │  stack: CallStack      - Current call frames                │    │
│  │  registers: [Value]    - Working values                     │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                     │
│  Memory:                                                            │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  heap_segments: Vec<Segment>  - Dynamic memory segments     │    │
│  │  heap_used: usize             - Bytes allocated             │    │
│  │  gc_generation: u8            - Young/old generation        │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                     │
│  Communication:                                                     │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  mailbox: Queue<Message>      - Incoming messages           │    │
│  │  waiting_pattern: Option<Pattern> - What we're waiting for  │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Process Memory

Each process has its own heap, allocated from the realm's process pool as dynamic segments:

```
PROCESS HEAP (Dynamic Segments)
────────────────────────────────────────────────────────────────────────

Initial spawn: 4 KB segment
┌────────────────────────────────────────────────────────────────────┐
│  Segment 0: 4 KB                                                   │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │  [locals] [call frames] [temporaries] ... [free]           │    │
│  └────────────────────────────────────────────────────────────┘    │
└────────────────────────────────────────────────────────────────────┘

After growth: multiple segments (BEAM-style)
┌────────────────────────────────────────────────────────────────────┐
│  Segment 0: 4 KB   (full)                                          │
│  Segment 1: 16 KB  (full)                                          │
│  Segment 2: 64 KB  (partial)                                       │
│  Segment 3: 256 KB (current allocation target)                     │
└────────────────────────────────────────────────────────────────────┘

Segments grow exponentially (4K → 16K → 64K → 256K → 1M → ...)
Old segments may be compacted during GC.
```

---

## Scheduling

Lonala uses a hybrid cooperative/preemptive scheduling model.

### Reduction-Based Scheduling

Within a realm, processes yield cooperatively after executing a certain number of "reductions" (bytecode instructions):

```
REDUCTION COUNTING
════════════════════════════════════════════════════════════════════════

const MAX_REDUCTIONS: u32 = 4000;  // Tune for ~1ms time slice

fn run_process(proc: &mut Process) -> RunResult {
    while proc.reductions > 0 {
        let instruction = fetch(proc);

        match execute(proc, instruction) {
            Continue => proc.reductions -= 1,
            Yield => return RunResult::Yielded,
            Block(reason) => return RunResult::Blocked(reason),
            Exit(reason) => return RunResult::Exited(reason),
        }
    }

    // Out of reductions - yield to other processes
    RunResult::Yielded
}
```

### Scheduler Loop

```
VM SCHEDULER LOOP
════════════════════════════════════════════════════════════════════════

loop {
    // Pick next runnable process
    proc = run_queue.pop_front()

    if proc is None {
        // No runnable processes - check mailboxes, maybe idle
        check_timeouts()
        if run_queue.is_empty() {
            wait_for_event()  // Block until message/timeout/signal
        }
        continue
    }

    // Reset reduction counter
    proc.reductions = MAX_REDUCTIONS

    // Run the process
    result = run_process(proc)

    match result {
        Yielded =>
            // Process used its time slice, re-queue
            run_queue.push_back(proc)

        Blocked(mailbox) =>
            // Process is waiting for a message
            waiting_processes.insert(proc.pid, proc)

        Exited(reason) =>
            // Process terminated
            notify_links(proc, reason)
            notify_monitors(proc, reason)
            cleanup(proc)
    }
}
```

### Scheduling Layers

```
THREE SCHEDULING LAYERS
════════════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────────┐
│ Layer 3: INTRA-REALM PROCESS SCHEDULING                             │
│                                                                     │
│   Mechanism: Reduction counting + run queue                         │
│   Granularity: ~1ms time slices                                     │
│   Fairness: Round-robin among runnable processes                    │
│   Control: Lona VM (userspace)                                      │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│ Layer 2: INTRA-REALM WORKER SCHEDULING (if multiple workers)        │
│                                                                     │
│   Mechanism: Work stealing between workers                          │
│   Granularity: Per-process                                          │
│   Fairness: Load balancing across CPUs                              │
│   Control: Lona VM (userspace)                                      │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│ Layer 1: INTER-REALM SCHEDULING                                     │
│                                                                     │
│   Mechanism: seL4 MCS scheduler                                     │
│   Granularity: Per-realm CPU budgets                                │
│   Fairness: Policy-defined (min/max budgets)                        │
│   Control: seL4 kernel (hardware-enforced)                          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

Key insight: Processes cooperate within a realm (reductions),
but the kernel preempts realms (MCS budgets). A misbehaving
process can't starve its realm's other processes, and a
misbehaving realm can't starve other realms.
```

### Work Stealing (Multi-Worker)

If a realm has multiple workers (TCBs), they can steal work from each other:

```
WORK STEALING (Optional, if multiple workers per realm)
════════════════════════════════════════════════════════════════════════

Each worker has a local run queue (deque):
- Owner pushes/pops from BOTTOM (LIFO - cache locality)
- Thieves steal from TOP (FIFO - oldest work)

fn worker_loop(worker_id):
    loop {
        // Try local queue first
        proc = local_queue.pop_bottom()

        if proc is None {
            // Try stealing from other workers
            proc = steal_from_others(worker_id)
        }

        if proc is None {
            // All queues empty - wait for work
            wait_for_work()
            continue
        }

        run_process(proc)
    }

fn steal_from_others(thief_id) -> Option<Process>:
    // Random start to avoid thundering herd
    start = random() % num_workers

    for i in 0..num_workers:
        victim = (start + i) % num_workers
        if victim == thief_id:
            continue

        if let Some(proc) = workers[victim].queue.steal_top():
            return Some(proc)

    None
```

---

## Message Passing

Processes communicate exclusively through message passing. No shared mutable state.

### Intra-Realm Messages (Fast Path)

Messages within the same realm use deep copy to the receiver's heap:

```
INTRA-REALM MESSAGE PASSING
════════════════════════════════════════════════════════════════════════

Cost: ~100-500 ns (deep copy)

sender                              receiver
   │                                   │
   │  (send receiver-pid [:ok data])   │
   │                                   │
   ├──────────────────────────────────▶│
   │                                   │
   │  1. Deep copy message to          │
   │     receiver's heap               │
   │                                   │
   │  2. Enqueue in receiver's         │
   │     mailbox (lock-free MPSC)      │
   │                                   │
   │  3. If receiver waiting,          │
   │     wake it up                    │
   │                                   │

Deep copy ensures:
- No shared mutable state
- Receiver owns message completely
- Sender can modify original after send
- GC of sender doesn't affect receiver
```

### Inter-Realm Messages (Kernel Path)

Messages between realms require seL4 IPC and serialization:

```
INTER-REALM MESSAGE PASSING
════════════════════════════════════════════════════════════════════════

Cost: ~1-10 µs (serialization + IPC)

Realm A                  seL4 Kernel              Realm B
   │                         │                       │
   │  (send pid [:ok data])  │                       │
   │                         │                       │
   │  1. Serialize message   │                       │
   │     to IPC buffer       │                       │
   │                         │                       │
   ├─────seL4_Call──────────▶│                       │
   │                         │                       │
   │                         ├──────seL4_Recv───────▶│
   │                         │                       │
   │                         │  2. Deserialize from  │
   │                         │     IPC buffer        │
   │                         │                       │
   │                         │  3. Deep copy to      │
   │                         │     receiver's heap   │
   │                         │                       │

Inter-realm IPC requires:
- Realm A has endpoint capability for Realm B
- Serialization format (TBD - likely compact binary)
- Deserialization + deep copy on receive
```

### Mailbox Implementation

Each process has a lock-free MPSC (multiple-producer, single-consumer) queue:

```
MAILBOX (Lock-Free MPSC Queue)
════════════════════════════════════════════════════════════════════════

struct Mailbox {
    head: AtomicPtr<Message>,  // Producers push here
    tail: *mut Message,        // Consumer pops here
}

struct Message {
    next: *mut Message,
    sender: ProcessId,
    data: Value,  // Deep-copied to receiver's heap
}

Push (any process can send):
────────────────────────────────────────────────────────────────────────
fn push(mailbox, msg):
    msg.next = null
    prev = atomic_exchange(&mailbox.head, msg)
    prev.next = msg  // Linearization point

Pop (only owner process):
────────────────────────────────────────────────────────────────────────
fn pop(mailbox) -> Option<Message>:
    tail = mailbox.tail
    next = tail.next

    if next is null:
        return None

    mailbox.tail = next
    return Some(next)
```

### Selective Receive

Processes can wait for messages matching a pattern:

```lonala
;; Wait for specific message pattern
(receive
  [:ok result]    (handle-success result)
  [:error reason] (handle-error reason)
  :timeout 5000   (handle-timeout))

;; Messages not matching patterns stay in mailbox
;; for later receive calls (selective receive)
```

```
SELECTIVE RECEIVE
════════════════════════════════════════════════════════════════════════

Process mailbox: [msg1] [msg2] [msg3] [msg4]

(receive [:ok result] ...)

1. Check msg1 against pattern [:ok result]
   - No match, skip

2. Check msg2 against pattern [:ok result]
   - Match! Extract result, remove msg2

3. Return to process with bound 'result'

Mailbox after: [msg1] [msg3] [msg4]
               (msg2 was consumed)

Non-matching messages remain for future receives.
```

### Large Message Optimization

Large binaries use reference counting instead of deep copy:

```
LARGE BINARY OPTIMIZATION
════════════════════════════════════════════════════════════════════════

Small values (< threshold): Deep copy
Large binaries (≥ threshold): Reference count + share

struct Binary {
    refcount: AtomicU32,
    size: usize,
    data: [u8],
}

When sending large binary:
1. Increment refcount
2. Copy pointer (not data) to receiver's heap
3. Both processes reference same physical data

When process exits or binary becomes unreachable:
1. Decrement refcount
2. If refcount == 0, free the binary

This makes sending large data efficient while maintaining
the "no shared mutable state" invariant (binaries are immutable).
```

---

## Garbage Collection

Each process has independent garbage collection - no global stop-the-world pauses.

### Per-Process GC

```
PER-PROCESS GARBAGE COLLECTION
════════════════════════════════════════════════════════════════════════

Key properties:
- Each process GC'd independently
- No global pauses
- Generational (young/old)
- Incremental (interleaved with execution)

Process A          Process B          Process C
    │                  │                  │
    │ GC running       │ executing        │ executing
    │ ████████         │                  │
    │                  │                  │
    │ executing        │ GC running       │ executing
    │                  │ ████████         │
    │                  │                  │
    │ executing        │ executing        │ GC running
    │                  │                  │ ████████
    │                  │                  │

No coordination needed between processes.
GC pauses only affect individual process (~microseconds).
```

### GC Triggers

```
GC TRIGGER CONDITIONS
════════════════════════════════════════════════════════════════════════

1. Allocation threshold:
   if process.heap_used > process.gc_threshold:
       trigger_gc(process)

2. Reduction-based (periodic):
   Every N reductions, check if GC beneficial

3. Explicit:
   (gc)  ; Force GC in current process

4. Memory pressure:
   If realm approaching memory limit, GC more aggressively
```

### Generational Collection

```
GENERATIONAL GC
════════════════════════════════════════════════════════════════════════

Young generation (nursery):
- Recently allocated objects
- Collected frequently (minor GC)
- Most objects die young → fast collection

Old generation:
- Objects that survived multiple minor GCs
- Collected less frequently (major GC)
- More expensive but less frequent

Minor GC (~10-100 µs):
- Scan young generation only
- Copy survivors to old generation
- Very fast, happens often

Major GC (~100 µs - 1 ms):
- Scan entire heap
- Compact old generation
- Less frequent, more expensive

Process heap after GC:
┌────────────────────────────────────────────────────────────────────┐
│  Old Generation (compacted)  │  Young Generation (empty)           │
│  [live] [live] [live]        │  [free space for allocation]        │
└────────────────────────────────────────────────────────────────────┘
```

---

## Process Linking and Monitoring

Processes can be notified when other processes exit.

### Links (Bidirectional)

```
PROCESS LINKS
════════════════════════════════════════════════════════════════════════

(spawn-link (fn [] (worker-loop)))

Creates bidirectional link:

Process A ←────link────→ Process B

If A crashes:
  B receives exit signal (crashes too, unless trapping)

If B crashes:
  A receives exit signal (crashes too, unless trapping)

Use case: Supervisor trees, coordinated shutdown
```

### Monitors (Unidirectional)

```
PROCESS MONITORS
════════════════════════════════════════════════════════════════════════

(spawn-monitor (fn [] (worker-loop)))

Creates unidirectional monitor:

Process A ────monitors────→ Process B

If B crashes:
  A receives [:DOWN ref pid reason] message

If A crashes:
  Nothing happens to B (unidirectional)

Use case: Watching without crashing together
```

### Exit Signals

```
EXIT SIGNAL PROPAGATION
════════════════════════════════════════════════════════════════════════

Process exits with reason:

Normal exit (:normal):
  - Links NOT notified (clean shutdown)
  - Monitors receive [:DOWN ref pid :normal]

Crash exit (:error, exception, etc.):
  - Links receive exit signal
  - Linked processes crash (unless trapping exits)
  - Monitors receive [:DOWN ref pid reason]

Trapping exits:
  (process-flag :trap-exit true)

  - Converts exit signals to messages
  - Process receives [:EXIT from-pid reason]
  - Used by supervisors to handle child crashes
```

---

## Summary

| Aspect | Description |
|--------|-------------|
| **Process creation** | ~1-10 µs, pure userspace |
| **Memory model** | Per-process heap, no shared mutable state |
| **Communication** | Message passing only, deep copy |
| **Intra-realm latency** | ~100-500 ns (deep copy) |
| **Inter-realm latency** | ~1-10 µs (serialization + IPC) |
| **Scheduling** | Reduction-based cooperative + MCS preemptive |
| **GC** | Per-process, generational, no global pauses |
| **Fault isolation** | Crash affects only linked processes |
