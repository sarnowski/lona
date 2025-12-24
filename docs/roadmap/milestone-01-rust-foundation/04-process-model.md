## Phase 1.4: Process Model

Implement BEAM-style lightweight processes.

### Design Goal: Process-Level Crash Semantics

A critical goal of the process model is **process-level crash isolation**. When a process exhausts its heap (OOM), only that process crashes—not the entire VM or domain. This follows BEAM semantics:

1. **Per-process heaps**: Each process allocates from its own heap
2. **OOM = process death**: When allocation fails, the allocator marks the process as dying
3. **Supervisor restart**: The supervisor detects the crash and restarts the process
4. **No Result propagation**: Code does NOT need to return `Result<T, AllocError>` everywhere; the allocator handles OOM by terminating the process

This design means:
- The shared compiler/VM code uses normal allocation (`Box::new`, `Vec::push`, etc.)
- The per-process allocator intercepts OOM and terminates the current process
- The root process (trusted code) crashing still takes down the system—this is acceptable since it only runs trusted code
- Untrusted code in child processes/domains can OOM safely without affecting others

---

### Task 1.4.1: Process Data Structure

**Description**: Define the process control block (PCB).

**Files to modify**:
- `crates/lona-kernel/src/process/mod.rs` (new)
- `crates/lona-kernel/src/process/pcb.rs` (new)

**Requirements**:
- PID (globally unique identifier)
- Status (running, waiting, suspended, terminated)
- Priority level
- Heap reference
- Stack/registers state
- Mailbox reference
- Links and monitors
- Reduction counter
- Current namespace
- Domain reference

**Tests**:
- PCB creation
- Status transitions
- Field accessors

**Estimated effort**: 1 context window

---

### Task 1.4.2: Per-Process Heap

**Description**: Implement isolated heap per process with OOM-triggered process termination.

**Files to modify**:
- `crates/lona-kernel/src/memory/heap.rs` (new)
- `crates/lona-kernel/src/process/pcb.rs`

**Requirements**:
- Each process has own heap allocator
- Heap grows on demand (within domain limits)
- Values allocated in owning process's heap
- Cross-process references require copying
- **OOM handling**: When allocation fails, mark process as `dying` with reason `OutOfMemory`
- **No panic on OOM**: Allocator must not panic; instead it signals process death
- **Safe points**: VM checks process status at reduction boundaries and handles death gracefully

**Tests**:
- Heap creation per process
- Allocation in process heap
- Heap isolation verification
- Heap growth
- OOM triggers process death (not panic)
- Process death reason is `OutOfMemory`

**Estimated effort**: 1-2 context windows

---

### Task 1.4.3: Process Registry

**Description**: Track all processes and enable lookup.

**Files to modify**:
- `crates/lona-kernel/src/process/registry.rs` (new)

**Requirements**:
- Global PID → Process mapping
- Named process registration
- Process enumeration
- Dead process cleanup

**Tests**:
- Process registration
- Name lookup
- PID lookup
- Cleanup on termination

**Estimated effort**: 1 context window

---

### Task 1.4.4: Mailbox Implementation

**Description**: Per-process message queue.

**Files to modify**:
- `crates/lona-kernel/src/process/mailbox.rs` (new)

**Requirements**:
- FIFO queue of messages
- Messages are copied Values
- Save queue for selective receive
- Timeout support

**Tests**:
- Message enqueue
- Message dequeue
- Save queue operation
- Empty mailbox behavior

**Estimated effort**: 1 context window

---

### Task 1.4.5: Scheduler - Run Queue

**Description**: Implement run queue for process scheduling.

**Files to modify**:
- `crates/lona-kernel/src/scheduler/mod.rs` (new)
- `crates/lona-kernel/src/scheduler/queue.rs` (new)

**Requirements**:
- Priority-based run queues
- O(1) enqueue/dequeue
- Process state transitions
- Fair scheduling within priority

**Tests**:
- Enqueue/dequeue operations
- Priority ordering
- State transition handling

**Estimated effort**: 1 context window

---

### Task 1.4.6: Scheduler - Context Switching

**Description**: Save/restore process execution state.

**Files to modify**:
- `crates/lona-kernel/src/scheduler/context.rs` (new)
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- Save VM state (IP, registers, stack frames)
- Restore VM state on resume
- Minimal overhead for switch
- Handle mid-instruction yields

**Tests**:
- Context save/restore roundtrip
- Multiple process interleaving
- State integrity verification

**Estimated effort**: 2 context windows

---

### Task 1.4.7: Scheduler - Cooperative Yielding

**Description**: Implement yield points for cooperative scheduling.

**Files to modify**:
- `crates/lona-kernel/src/scheduler/mod.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- Yield on `receive` when no matching message
- Yield on explicit `yield` call
- Yield on timer expiration
- Resume when condition met

**Tests**:
- Yield on empty receive
- Yield on explicit call
- Resume on message arrival

**Estimated effort**: 1 context window

---

### Task 1.4.8: Scheduler - Preemptive Scheduling

**Description**: Implement reduction counting for preemption.

**Files to modify**:
- `crates/lona-kernel/src/scheduler/mod.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- Count reductions (roughly: bytecode ops)
- Preempt after N reductions (configurable)
- No process can monopolize CPU
- Reduction count visible for debugging

**Tests**:
- Preemption after reduction limit
- Fair time slicing
- Reduction counter accuracy

**Estimated effort**: 1 context window

---

### Task 1.4.9: Spawn Primitive

**Description**: Implement `spawn` to create new processes.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- `(spawn fn)` creates process running fn
- `(spawn fn args)` with initial arguments
- Returns PID immediately
- New process starts in runnable state

**Tests**:
- Basic spawn
- Spawn with arguments
- PID uniqueness
- Process starts execution

**Estimated effort**: 1 context window

---

### Task 1.4.10: Self and Exit Primitives

**Description**: Implement `self` and `exit`.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- `(self)` returns current process PID
- `(exit reason)` terminates with reason
- Exit reason sent to linked processes
- Cleanup process resources

**Tests**:
- Self returns correct PID
- Normal exit
- Exit with error reason
- Resource cleanup

**Estimated effort**: 1 context window

---

### Task 1.4.11: Send Primitive - Intra-Domain

**Description**: Implement message sending within same domain.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- `(send pid msg)` copies message to target mailbox
- Deep copy of message (process isolation)
- Wake target if waiting for message
- Returns message (for chaining)

**Tests**:
- Basic send
- Message copying verification
- Wake on send
- Send to self

**Estimated effort**: 1 context window

---

### Task 1.4.12: Receive Special Form - Basic

**Description**: Implement basic `receive` with patterns.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/mod.rs`
- `crates/lonala-compiler/src/compiler/receive.rs` (new)
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- `(receive pattern1 body1 pattern2 body2 ...)`
- Pattern match against mailbox messages
- First matching message removed from mailbox
- Blocks if no match (yield to scheduler)

**Tests**:
- Simple pattern receive
- Multiple patterns
- Pattern with binding
- Blocking behavior

**Estimated effort**: 2 context windows

---

### Task 1.4.13: Receive with Timeout

**Description**: Add timeout support to `receive`.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/receive.rs`
- `crates/lona-kernel/src/scheduler/timer.rs` (new)

**Requirements**:
- `(receive ... (after ms expr))` with timeout
- Timer integration with scheduler
- Timeout triggers alternative expression
- Cancel timer if message received

**Tests**:
- Receive with timeout (timeout triggers)
- Receive with timeout (message arrives first)
- Zero timeout (poll)
- Multiple timers

**Estimated effort**: 1-2 context windows

---

### Task 1.4.14: Selective Receive

**Description**: Implement BEAM-style selective receive with save queue.

**Files to modify**:
- `crates/lona-kernel/src/process/mailbox.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- Non-matching messages saved for later
- After receive, save queue restored to mailbox
- Preserves message ordering
- Avoids re-scanning saved messages

**Tests**:
- Selective receive skips non-matching
- Save queue restored
- Message ordering preserved
- Performance with many messages

**Estimated effort**: 1 context window

---

### Task 1.4.15: Per-Process Binding Stack

**Description**: Each process maintains a stack of dynamic binding frames.

**Files to modify**:
- `crates/lona-kernel/src/process/pcb.rs`
- `crates/lona-kernel/src/process/bindings.rs` (new)

**Requirements**:
- Binding frame: Map of Var → Value
- Stack of frames per process
- Lookup checks frames top-down, falls back to root value
- Push/pop frame operations
- Frame automatically popped on scope exit

**Tests**:
- Push binding frame
- Lookup finds bound value
- Pop restores previous
- Nested frames work correctly

**Estimated effort**: 1 context window

---

### Task 1.4.16: `binding` Special Form

**Description**: Implement `binding` to establish dynamic bindings in scope.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/mod.rs`
- `crates/lonala-compiler/src/compiler/binding.rs` (new)
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- `(binding [*out* new-out] body...)` syntax
- Pushes frame before body, pops after (including on error)
- Only dynamic vars can be bound (compile-time check)
- Bindings visible to all code called from body
- New opcodes: `PushBindingFrame`, `PopBindingFrame`

**Tests**:
- Simple binding
- Nested bindings
- Binding visible in called functions
- Frame popped on normal exit
- Frame popped on error exit

**Estimated effort**: 1-2 context windows
