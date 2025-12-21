## Phase 1.11: Introspection System

Implement LISP-machine-style introspection and debugging capabilities as described in `goals.md`.

---

### Task 1.11.1: Source Storage and Retrieval

**Description**: Store source code per-definition with provenance tracking.

**Files to modify**:
- `crates/lona-kernel/src/namespace/mod.rs`
- `crates/lona-kernel/src/vm/globals.rs`

**Requirements**:
- Each definition stores original source text
- Store provenance: file, line, timestamp, previous version chain
- Comments preceding definition attached to it
- Source accessible at runtime

**Tests**:
- Definition stores source
- Provenance tracked
- Comments preserved

**Estimated effort**: 1-2 context windows

---

### Task 1.11.2: `source` and `disassemble` Functions

**Description**: View source code and bytecode of definitions.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/vm/introspection.rs`

**Requirements**:
- `(source fn)` - returns source string with provenance header
- `(disassemble fn)` - returns bytecode representation
- Works for any function or var
- Shows REPL vs file origin

**Tests**:
- Source of file-defined function
- Source of REPL-defined function
- Disassemble output format

**Estimated effort**: 1 context window

---

### Task 1.11.3: Namespace Introspection

**Description**: Query namespace contents and metadata.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/namespace/mod.rs`

**Requirements**:
- `(ns-map ns)` - all mappings in namespace
- `(ns-publics ns)` - public vars only
- `(ns-interns ns)` - vars defined in this ns
- `(ns-refers ns)` - referred vars from other ns
- `(all-ns)` - list all namespaces

**Tests**:
- Query various namespace contents
- Public vs private distinction
- Referred vars listed

**Estimated effort**: 1 context window

---

### Task 1.11.4: Process Introspection

**Description**: Inspect process state and metadata.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- `(process-info pid)` - returns map with pid, name, status, heap-size, etc.
- `(process-state pid)` - get process internal state
- `(process-messages pid)` - view mailbox contents
- `(list-processes)` - enumerate all processes

**Tests**:
- Info for running process
- Info for waiting process
- Message queue inspection

**Estimated effort**: 1-2 context windows

---

### Task 1.11.5: Domain Introspection

**Description**: Query domain hierarchy and capabilities.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/domain/mod.rs`

**Requirements**:
- `(domain-of pid)` - get domain name for process
- `(domain-info name)` - returns map with parent, capabilities, processes, memory
- `(domain-meta name)` - get domain metadata
- `(list-domains)` - enumerate all domains
- `(find-domains query)` - find domains matching metadata query
- `(same-domain? pid1 pid2)` - check if same domain

**Tests**:
- Domain lookup
- Metadata query
- Parent/child relationships

**Estimated effort**: 1-2 context windows

---

### Task 1.11.6: Tracing Infrastructure

**Description**: Non-blocking observation of system behavior.

**Files to modify**:
- `crates/lona-kernel/src/vm/tracing.rs` (new)
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(trace-calls fn opts)` - trace function invocations
- `(trace-messages pid opts)` - trace message send/receive
- `(untrace fn)` - stop tracing
- Trace output includes timestamps
- Minimal performance overhead

**Tests**:
- Trace function calls
- Trace message passing
- Untrace stops output

**Estimated effort**: 2 context windows

---

### Task 1.11.7: Hot Code Propagation

**Description**: Explicit code updates between domains.

**Files to modify**:
- `crates/lona-kernel/src/domain/mod.rs`
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(push-code domain fn-name)` - push updated function to child domain
- `(pull-code domain fn-name)` - pull updated function from parent
- `(on-code-push handler)` - register handler for incoming pushes
- Capability-controlled access

**Tests**:
- Push code to child
- Pull code from parent
- Handler can accept/reject

**Estimated effort**: 1-2 context windows
