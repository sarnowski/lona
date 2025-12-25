# Phase 1.10: Condition/Restart System

Implement Common Lisp-inspired condition system for recoverable errors.

---

## Task 1.10.1: Condition Type and Signal

**Description**: Define condition representation and basic signaling.

**Files to modify**:
- `crates/lona-core/src/value/condition.rs` (new)
- `crates/lona-core/src/value/mod.rs`
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- Condition is a map with at least `:type` key
- `(signal condition)` raises condition without unwinding
- If no handler, becomes process exit with condition as reason
- Condition carries arbitrary data for context
- `condition?` predicate

**Tests**:
- Create condition
- Signal with no handler (process exits)
- Condition data accessible
- Predicate works

**Estimated effort**: 1 context window

---

## Task 1.10.2: Handler Binding Infrastructure

**Description**: Dynamic binding mechanism for condition handlers.

**Files to modify**:
- `crates/lona-kernel/src/process/conditions.rs` (new)
- `crates/lona-kernel/src/process/pcb.rs`

**Requirements**:
- Handler stack per process (similar to binding stack)
- Handler: `{:type type-keyword :fn handler-fn}`
- Multiple handlers can be bound for same type (most recent wins)
- `find-handler` searches stack for matching type
- Handlers receive condition, can inspect without unwinding

**Tests**:
- Push handler
- Find handler by type
- Most recent handler wins
- No handler returns nil

**Estimated effort**: 1 context window

---

## Task 1.10.3: `handler-bind` Macro

**Description**: Establish condition handlers for a body of code.

**Files to create**:
- `lona/core/conditions.lona`

**Requirements**:
- `(handler-bind [type handler-fn ...] body)`
- Pushes handlers before body, pops after
- Handler function receives condition map
- Handler can: invoke restart, re-signal, return value
- Multiple type/handler pairs supported

**Tests**:
- Handler called on matching condition
- Handler not called on non-matching
- Multiple handlers
- Handler can access condition data

**Estimated effort**: 1 context window

---

## Task 1.10.4: Restart Registry

**Description**: Per-signal-point restart registration.

**Files to modify**:
- `crates/lona-kernel/src/process/conditions.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- When condition signaled, restarts are registered
- Restart: `{:name keyword :fn restart-fn :description string}`
- Restarts stored in condition context (not global)
- `available-restarts` returns current restarts
- Restarts cleared when condition handled

**Tests**:
- Register restarts with signal
- List available restarts
- Restarts cleared after handling

**Estimated effort**: 1 context window

---

## Task 1.10.5: `restart-case` Macro

**Description**: Establish restarts for potentially-signaling code.

**Files to modify**:
- `lona/core/conditions.lona`

**Requirements**:
- ```clojure
  (restart-case expr
    (:retry [] "Try again" (retry-logic))
    (:use-value [v] "Use provided value" v))
  ```
- Each restart becomes a continuation point
- Restart functions receive args from `invoke-restart`
- Descriptions available for interactive selection

**Tests**:
- Define restarts
- Restarts available during signal
- Restart descriptions accessible
- Multiple restarts

**Estimated effort**: 1-2 context windows

---

## Task 1.10.6: `invoke-restart` Function

**Description**: Choose and invoke a restart from handler.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/process/conditions.rs`

**Requirements**:
- `(invoke-restart :restart-name args...)`
- Looks up restart by name
- Transfers control to restart point (non-local jump)
- Passes args to restart function
- Stack unwound only to restart point, not further

**Tests**:
- Invoke restart by name
- Args passed correctly
- Control transfers to restart point
- Stack properly unwound

**Estimated effort**: 2 context windows

---

## Task 1.10.7: Basic Condition REPL Integration

**Description**: Show unhandled conditions in REPL and allow restart selection.

**Files to modify**:
- `crates/lona-runtime/src/repl.rs`

**Requirements**:
- When unhandled condition reaches REPL, show formatted error
- Display condition type and data
- List available restarts with descriptions
- User can type restart number to select
- Basic `:abort` restart returns to REPL prompt

**Note**: This is the minimal integration for conditions in the REPL. Phase 1.12 extends this with full debug mode (attach/detach, breakpoints, stepping, stack inspection).

**Tests**:
- Unhandled condition shows error and restarts
- User can select restart by number
- Abort returns to REPL
- Condition data displayed

**Estimated effort**: 1 context window
