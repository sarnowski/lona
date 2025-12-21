## Phase 1.5: Garbage Collection

Implement per-process incremental garbage collection.

---

### Task 1.5.1: Root Discovery

**Description**: Identify GC roots for a process.

**Files to modify**:
- `crates/lona-kernel/src/gc/mod.rs` (new)
- `crates/lona-kernel/src/gc/roots.rs` (new)

**Requirements**:
- Stack frame locals are roots
- Global vars in process's namespace are roots
- Mailbox messages are roots
- Closures' captured values are roots

**Tests**:
- Root enumeration
- All root types discovered
- No roots missed

**Estimated effort**: 1 context window

---

### Task 1.5.2: Tri-Color Marking

**Description**: Implement tri-color marking algorithm.

**Files to modify**:
- `crates/lona-kernel/src/gc/marker.rs` (new)

**Requirements**:
- White (unvisited), Gray (pending), Black (done)
- Incremental: configurable work per step
- Track object color efficiently
- Handle cycles correctly

**Tests**:
- Simple object graph marking
- Cyclic structures
- Incremental progress
- All reachable marked black

**Estimated effort**: 1-2 context windows

---

### Task 1.5.3: Write Barrier

**Description**: Implement write barrier for incremental correctness.

**Files to modify**:
- `crates/lona-core/src/value/mod.rs`
- `crates/lona-kernel/src/gc/barrier.rs` (new)

**Requirements**:
- Detect when black object points to white
- Mark target gray (or re-mark source)
- Minimal overhead on writes
- Works with all mutable operations

**Tests**:
- Barrier triggered on mutation
- Correctness maintained during mutation
- Overhead measurement

**Estimated effort**: 1-2 context windows

---

### Task 1.5.4: Sweep Phase

**Description**: Reclaim unmarked memory.

**Files to modify**:
- `crates/lona-kernel/src/gc/sweep.rs` (new)
- `crates/lona-kernel/src/memory/heap.rs`

**Requirements**:
- Identify unmarked (white) objects
- Return memory to heap
- Update heap statistics
- Handle finalizers (if any)

**Tests**:
- Sweep reclaims unreachable
- Memory returned to heap
- Statistics accuracy

**Estimated effort**: 1 context window

---

### Task 1.5.5: Generational Optimization

**Description**: Add generational collection for reduced pause times.

**Files to modify**:
- `crates/lona-kernel/src/gc/mod.rs`
- `crates/lona-kernel/src/gc/generations.rs` (new)

**Requirements**:
- Young generation (frequently collected)
- Old generation (rarely collected)
- Promotion after N survivals
- Remember set for old→young references

**Tests**:
- Young generation collection
- Promotion to old
- Remember set accuracy
- Full collection when needed

**Estimated effort**: 2-3 context windows

---

### Task 1.5.6: GC Scheduling

**Description**: Determine when to run GC.

**Files to modify**:
- `crates/lona-kernel/src/gc/scheduler.rs` (new)
- `crates/lona-kernel/src/scheduler/mod.rs`

**Requirements**:
- Trigger on allocation pressure
- Incremental work between process yields
- Per-process isolation (one process's GC doesn't affect others)
- `gc` and `gc-stats` primitives

**Tests**:
- GC triggered on pressure
- Incremental progress
- Process isolation
- Statistics accuracy

**Estimated effort**: 1-2 context windows
