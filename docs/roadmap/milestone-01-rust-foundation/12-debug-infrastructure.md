# Phase 1.12: Debug Infrastructure

Implement the Two-Mode Architecture for LISP-machine-style debugging within BEAM/OTP-style resilience. See [docs/lonala/debugging.md](../../lonala/debugging.md) for full specification.

**Dependencies**: Phase 1.10 (Condition/Restart System), Phase 1.11 (Introspection System)

**Relationship to Phase 1.10**: Phase 1.10 provides the condition/restart mechanism and basic REPL integration. Phase 1.12 extends this with the Two-Mode Architecture: production mode (crash on error) vs debug mode (pause on error), debugger attach/detach, breakpoints, and stepping.

---

## Task 1.12.1: Process Debug State

**Description**: Add debug mode flag and `:debugging` state to processes.

**Files to modify**:
- `crates/lona-kernel/src/process/pcb.rs`
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- Add `debug_mode: bool` flag to Process struct
- Add `:debugging` to process state enum
- Add `debug_channel: Option<Channel>` for debug commands
- Supervisor recognizes `:debugging` state (doesn't restart)
- Process enters `:debugging` when debugger attached and error occurs

**Tests**:
- Process state transitions include `:debugging`
- Supervisor ignores debugged processes
- Debug mode flag toggles correctly

**Estimated effort**: 1 context window

---

## Task 1.12.2: Debug Attach/Detach

**Description**: Implement `debug-attach` and `debug-detach` primitives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- `(debug-attach pid)` - attach debugger, set debug mode
- `(debug-detach pid)` - detach, return to production mode
- `(debug-attached? pid)` - check if debugger attached
- Requires debug capability for target domain
- Returns `:ok` or `{:error reason}`

**Tests**:
- Attach to own process
- Attach to process in same domain
- Capability enforcement for other domains
- Detach restores production mode

**Estimated effort**: 1 context window

---

## Task 1.12.3: Panic Behavior in Debug Mode

**Description**: Modify `panic!` to pause instead of crash when debugger attached.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/process/mod.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- In production mode: `panic!` terminates process immediately
- In debug mode: `panic!` pauses process, enters `:debugging` state
- Paused process sends debug event to attached debugger
- Debug event includes: condition, stack frames, locals, available restarts
- Standard restarts available: `:abort`, `:continue` (if possible)

**Tests**:
- Panic in production mode crashes
- Panic in debug mode pauses
- Debug event sent to debugger
- Abort restart crashes process
- Continue restart resumes (when applicable)

**Estimated effort**: 2 context windows

---

## Task 1.12.4: Stack Frame Reification

**Description**: Expose stack frames as inspectable values.

**Files to modify**:
- `crates/lona-core/src/value/mod.rs`
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- `(debug-frames pid)` - get list of frame maps for paused process
- Each frame: `{:index N :function sym :line N :file "path" :locals {...}}`
- `(debug-locals pid frame-idx)` - get locals map for specific frame
- `(debug-source pid frame-idx)` - get source code for frame
- Only works on paused/debugging processes
- Capability enforcement for cross-domain

**Tests**:
- Get frames of paused process
- Frame contains expected keys
- Locals map is accurate
- Source retrieval works

**Estimated effort**: 2 context windows

---

## Task 1.12.5: In-Frame Evaluation

**Description**: Evaluate expressions in the context of a specific stack frame.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- `(debug-eval pid frame-idx expr)` - evaluate expr in frame context
- Expression has access to frame's local variables
- Can call functions visible from that frame
- Returns evaluation result or error
- `(debug-set-local! pid frame-idx name value)` - modify local variable

**Tests**:
- Evaluate local variable reference
- Evaluate expression using locals
- Call function from frame context
- Modify local variable
- Error on invalid frame

**Estimated effort**: 2 context windows

---

## Task 1.12.6: Debug Control Operations

**Description**: Implement pause, continue, and stepping.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/scheduler/mod.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- `(debug-pause pid)` - externally pause a running process
- `(debug-continue pid)` - resume paused process
- `(debug-step pid)` - execute one expression, then pause
- `(debug-step-over pid)` - step but don't pause in called functions
- `(debug-step-out pid)` - continue until current function returns
- Stepping requires instruction-level bookkeeping

**Tests**:
- Pause running process
- Continue paused process
- Step executes one instruction
- Step-over skips called functions
- Step-out finishes current function

**Estimated effort**: 2-3 context windows

---

## Task 1.12.7: Breakpoint Infrastructure

**Description**: Implement pattern-matching breakpoints.

**Files to modify**:
- `crates/lona-kernel/src/vm/breakpoints.rs` (new)
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- Breakpoint types: `:call`, `:return`, `:receive`
- Pattern matching on arguments/return values/messages
- Optional guard expressions
- Actions: `:pause`, `:log`, `:trace`
- `(set-breakpoint type target opts)` - create breakpoint
- `(clear-breakpoint id)` - remove breakpoint
- `(list-breakpoints)` - enumerate active breakpoints

**Tests**:
- Entry breakpoint pauses on matching call
- Return breakpoint pauses on matching return value
- Pattern matching works correctly
- Guard expressions evaluated
- Clear removes breakpoint

**Estimated effort**: 3 context windows

---

## Task 1.12.8: Breakpoint via Dispatch Table

**Description**: Implement breakpoints using dispatch table trampolines.

**Files to modify**:
- `crates/lona-kernel/src/vm/breakpoints.rs`
- `crates/lona-kernel/src/vm/globals.rs`

**Requirements**:
- Original: `foo → bytecode-A`
- With breakpoint: `foo → trampoline → bytecode-A`
- Trampoline checks pattern, pauses if matched
- Return breakpoints wrap return path
- Minimal overhead when pattern doesn't match
- Per-domain breakpoints (don't affect other domains)

**Tests**:
- Trampoline installed correctly
- Pattern checking works
- Non-matching calls have minimal overhead
- Domain isolation maintained

**Estimated effort**: 2 context windows

---

## Task 1.12.9: Trace-to-Break Upgrade

**Description**: Convert non-blocking traces to blocking breakpoints.

**Files to modify**:
- `crates/lona-kernel/src/vm/tracing.rs`
- `crates/lona-kernel/src/vm/breakpoints.rs`

**Requirements**:
- `(trace-to-break trace-id)` - upgrade trace to breakpoint
- Trace continues logging until pattern matches
- On match, becomes blocking breakpoint
- User can then inspect and step
- `(break-to-trace breakpoint-id)` - downgrade to trace

**Tests**:
- Trace upgraded to breakpoint
- Pattern match triggers pause
- Downgrade returns to tracing

**Estimated effort**: 1 context window

---

## Task 1.12.10: Debugger REPL Integration

**Description**: Integrate debug mode with REPL interface.

**Files to modify**:
- `crates/lona-runtime/src/repl.rs`
- `lona/debugger.lona`

**Requirements**:
- When process pauses, switch REPL to debug mode
- Debug prompt: `proc-debug[frame]>`
- Commands: `l` (locals), `e` (eval), `u`/`d` (up/down), `c` (continue)
- Numeric input selects restart
- `q` detaches debugger
- Show formatted error/condition on pause

**Tests**:
- REPL enters debug mode on pause
- Commands work correctly
- Restart selection works
- Detach returns to normal REPL

**Estimated effort**: 2 context windows

---

## Task 1.12.11: Supervisor Debug Awareness

**Description**: Make supervisors aware of debug state.

**Files to modify**:
- `lona/supervisor.lona` (when M2 is implemented)
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- Supervisor checks for `:debugging` state before restart
- Optional `:debug-timeout` configuration
- Supervisor waits for debug to complete or timeout
- After timeout, can force-crash or continue waiting
- "Resume & Crash" option for testing supervisor recovery

**Tests**:
- Supervisor doesn't restart debugging process
- Timeout triggers configured action
- Force-crash works
- Resume & crash option available

**Estimated effort**: 1-2 context windows
