# Phase 1.7: Fault Tolerance

Implement Erlang-style fault tolerance mechanisms.

---

## Task 1.7.1: Process Linking

**Description**: Bidirectional process links for crash propagation.

**Files to modify**:
- `crates/lona-kernel/src/process/links.rs` (new)
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(link pid)` creates bidirectional link
- `(unlink pid)` removes link
- `(spawn-link fn)` atomic spawn+link
- Exit propagates to linked processes

**Tests**:
- Link creation
- Exit propagation
- Unlink stops propagation
- spawn-link atomicity

**Estimated effort**: 1-2 context windows

---

## Task 1.7.2: Process Monitoring

**Description**: Unidirectional monitoring without crash propagation.

**Files to modify**:
- `crates/lona-kernel/src/process/monitors.rs` (new)
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(monitor pid)` starts monitoring, returns ref
- `(demonitor ref)` stops monitoring
- `:DOWN` message on monitored process exit
- Monitor doesn't propagate crash

**Tests**:
- Monitor creation
- DOWN message delivery
- Demonitor stops messages
- No crash propagation

**Estimated effort**: 1-2 context windows

---

## Task 1.7.3: Exit Signals

**Description**: Implement exit signal delivery and trapping.

**Files to modify**:
- `crates/lona-kernel/src/process/signals.rs` (new)
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- Normal exit (`:normal`) doesn't crash linked
- Abnormal exit crashes linked (unless trapped)
- `(process-flag :trap-exit true)` enables trapping
- Trapped exits become messages

**Tests**:
- Normal exit behavior
- Abnormal exit propagation
- Exit trapping
- Trap exit messages

**Estimated effort**: 1-2 context windows

---

## Task 1.7.4: Panic Implementation

**Description**: Implement untrappable `panic!`.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- `(panic! msg)` terminates immediately
- `(panic! msg data)` with context
- Cannot be trapped
- Supervisor still notified

**Tests**:
- Panic terminates
- Cannot be trapped
- Supervisor notification

**Estimated effort**: 0.5 context windows

---

## Task 1.7.5: Cross-Domain Fault Tolerance

**Description**: Links and monitors work across domain boundaries.

**Files to modify**:
- `crates/lona-kernel/src/process/links.rs`
- `crates/lona-kernel/src/process/monitors.rs`
- `crates/lona-runtime/src/domain/ipc.rs`

**Requirements**:
- Link to process in another domain
- Monitor across domains
- Exit signals cross domains
- Domain crash affects all its processes

**Tests**:
- Cross-domain link
- Cross-domain monitor
- Domain crash handling

**Estimated effort**: 2 context windows
