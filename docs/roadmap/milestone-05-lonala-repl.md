## Milestone 5: Lonala REPL

**Goal**: Replace Rust REPL with pure Lonala implementation.

**Prerequisite**: Milestone 4 complete

### Phase 5.1: REPL Core

#### Task 5.1.1: REPL Main Loop

**Description**: Implement read-eval-print loop.

**Files to create**:
- `lona/repl.lona`

**Requirements**:
- `lona.repl/main` entry point
- Read from UART
- Parse input
- Evaluate expression
- Print result

**Estimated effort**: 1-2 context windows

---

#### Task 5.1.2: Line Editor

**Description**: Implement basic line editing.

**Files to modify**:
- `lona/repl.lona`

**Requirements**:
- Character-by-character input
- Backspace handling
- Line buffering
- Enter to submit

**Estimated effort**: 1 context window

---

### Phase 5.2: REPL Features

#### Task 5.2.1: Error Handling

**Description**: Handle and display errors gracefully.

**Files to modify**:
- `lona/repl.lona`

**Requirements**:
- Catch parse errors
- Catch compile errors
- Catch runtime errors
- Formatted error display

**Estimated effort**: 1 context window

---

#### Task 5.2.2: Multi-line Input

**Description**: Support multi-line expressions.

**Files to modify**:
- `lona/repl.lona`

**Requirements**:
- Detect incomplete expressions
- Continuation prompt
- Balanced delimiter tracking

**Estimated effort**: 1 context window

---

#### Task 5.2.3: History

**Description**: Implement command history.

**Files to modify**:
- `lona/repl.lona`

**Requirements**:
- Store previous inputs
- Up/down arrow navigation
- History search (Ctrl-R)

**Estimated effort**: 1 context window

---

### Phase 5.3: Integration

#### Task 5.3.1: REPL Domain

**Description**: Run REPL in isolated domain.

**Files to modify**:
- `lona/repl.lona`
- `lona/init.lona`

**Requirements**:
- Spawn REPL in own domain
- Connect to UART driver via IPC
- Appropriate capabilities

**Estimated effort**: 1 context window

---

#### Task 5.3.2: Remove Rust REPL

**Description**: Remove interim Rust REPL code.

**Files to modify**:
- `crates/lona-runtime/src/main.rs`
- `crates/lona-runtime/src/repl.rs` (delete)

**Requirements**:
- Remove Rust REPL implementation
- Boot directly to Lonala init
- Update documentation

**Estimated effort**: 1 context window

---

