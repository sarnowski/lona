## Phase 1.9: Integration & Spec Tests

Ensure all components work together and pass specification tests.

---

### Task 1.9.1: Spec Test Framework Enhancement

**Description**: Enhance spec test infrastructure for new features.

**Files to modify**:
- `crates/lona-spec-tests/src/lib.rs`
- Various test files

**Requirements**:
- Tests for all new value types
- Tests for all new primitives
- Tests for process operations
- Tests for domain operations

**Tests**:
- Meta: test framework tests itself

**Estimated effort**: 2-3 context windows

---

### Task 1.9.2: Process Integration Tests

**Description**: End-to-end tests for process model.

**Files to modify**:
- `crates/lona-spec-tests/src/processes.rs` (new)

**Requirements**:
- Spawn and communication
- Linking and monitoring
- Exit propagation
- Supervision patterns

**Tests**:
- Multi-process scenarios
- Fault tolerance scenarios

**Estimated effort**: 2 context windows

---

### Task 1.9.3: Domain Integration Tests

**Description**: End-to-end tests for domain isolation.

**Files to modify**:
- `crates/lona-spec-tests/src/domains.rs` (new)

**Requirements**:
- Domain creation
- Inter-domain messaging
- Capability transfer
- Isolation verification

**Tests**:
- Cross-domain scenarios
- Security boundary tests

**Estimated effort**: 2 context windows

---

### Task 1.9.4: GC Integration Tests

**Description**: Verify GC correctness under load.

**Files to modify**:
- `crates/lona-spec-tests/src/gc.rs` (new)

**Requirements**:
- Long-running allocation
- Cyclic structures
- Cross-generation references
- Concurrent GC with execution

**Tests**:
- Memory pressure scenarios
- Correctness verification

**Estimated effort**: 1-2 context windows

---

### Task 1.9.5: Full System Integration Test

**Description**: Boot complete system with all features.

**Files to modify**:
- `crates/lona-runtime/src/main.rs`
- Integration test files

**Requirements**:
- Boot to REPL
- All primitives available
- Process spawning works
- Domain creation works

**Tests**:
- Full boot sequence
- Feature availability

**Estimated effort**: 1-2 context windows

---

### Task 1.9.6: Hot Code Loading Tests

**Description**: Verify hot code loading works correctly.

**Files to create**:
- `crates/lona-spec-tests/src/hot_loading.rs`

**Requirements**:
- Test: redefine function, callers see new version immediately
- Test: recursive function redefined mid-recursion behaves correctly
- Test: closure captures see updated function references
- Test: long-running process sees redefined functions
- Verify dispatch table updates are atomic

**Tests**:
- Immediate caller update
- Recursive update
- Closure behavior
- Long-running process behavior

**Estimated effort**: 1 context window

---

### Task 1.9.7: Cross-Domain Code Isolation Tests

**Description**: Verify parent patches don't affect children.

**Files to create**:
- `crates/lona-spec-tests/src/domain_code_isolation.rs`

**Requirements**:
- Spawn child domain with copy of parent's dispatch table
- Redefine function in parent
- Verify child still sees old version
- Verify grandchild spawned from child sees child's version
- Test explicit `push-code` propagation when implemented

**Tests**:
- Parent patch doesn't affect child
- Child patch doesn't affect parent
- Grandchild inherits from child
- Explicit propagation works (when implemented)

**Estimated effort**: 1-2 context windows

---

### Task 1.9.8: Dynamic Binding Tests

**Description**: Test dynamic variable binding system.

**Files to create**:
- `crates/lona-spec-tests/src/dynamic_bindings.rs`

**Requirements**:
- Test `^:dynamic` var declaration
- Test `binding` special form establishes scope
- Test bindings visible in called functions
- Test nested bindings (inner shadows outer)
- Test per-process binding isolation
- Test frame pop on normal and error exits

**Tests**:
- Simple dynamic binding
- Nested bindings
- Cross-function visibility
- Process isolation
- Error cleanup

**Estimated effort**: 1 context window
