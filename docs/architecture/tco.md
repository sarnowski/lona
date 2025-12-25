# Proper Tail Calls (Scheme-Style TCO)

This document describes Lona's implementation of proper tail call elimination, following Scheme's model where tail call optimization is a **semantic guarantee**, not merely an optimization.

## Design Philosophy

### Why Full TCO Instead of `loop`/`recur`

Clojure uses explicit `loop`/`recur` because the JVM doesn't support tail call optimization. Rich Hickey's position was that partial TCO (sometimes optimized, sometimes not) is worse than no TCO because it creates unpredictable behavior.

Lona has no such constraint. Our bytecode VM has full control over the call stack. Following the Scheme/LISP tradition, we implement **proper tail calls**: any call in tail position is guaranteed not to consume stack space.

This means:
- No special keywords needed for iteration
- Mutual recursion works efficiently (state machines, parsers)
- Code is cleaner and more natural
- `loop`/`recur` can be added later as optional syntactic sugar if desired

### Compatibility with LISP Machine Debugging

TCO does NOT conflict with LISP machine-style debugging:

- **Tracing/advice system**: Works independently of stack
- **Condition/restart system**: Doesn't rely on stack unwinding
- **Source locations**: Preserved in bytecode
- **Step debugging**: Works at bytecode level

Scheme/Racket proves that full TCO and excellent debugging can coexist.

---

## Architecture

### Current Problem

```
User code: (fn [x] (if (zero? x) x (f (dec x))))
                                    ↓
Compiler: emits Call opcode (always)
                                    ↓
VM: op_call() → call_user_function() → recursive self.run()
                                    ↓
Result: Rust call stack grows with each Lonala call
```

### Target Architecture

```
User code: (fn [x] (if (zero? x) x (f (dec x))))
                                    ↓
Compiler: detects tail position → emits TailCall opcode
                                    ↓
VM: op_tail_call() → prepares new frame → returns TailCallRequest
                                    ↓
run() loop: receives TailCallRequest → swaps frame → continues loop
                                    ↓
Result: Single Rust stack frame, unbounded Lonala tail calls
```

---

## Implementation Tasks

### Task 1.2.6: Proper Tail Calls - Compiler

**Description**: Add tail position tracking to the compiler and emit `TailCall` opcode for calls in tail position.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/mod.rs`
- `crates/lonala-compiler/src/compiler/special_forms.rs`
- `crates/lonala-compiler/src/compiler/functions.rs`
- `crates/lonala-compiler/src/compiler/calls.rs`

**Requirements**:

1. Add `in_tail_position: bool` field to `Compiler` struct

2. Create internal method that accepts tail context:
   ```rust
   fn compile_expr_in_context(&mut self, expr: &Spanned<Ast>, tail: bool) -> Result<...>
   ```

3. Propagate tail position through special forms:
   - `fn` body: last expression is in tail position
   - `do`: last expression inherits caller's tail position
   - `if`: both branches inherit caller's tail position
   - `let`: body inherits caller's tail position
   - All other contexts: `tail = false`

4. Modify `compile_call()` to emit appropriate opcode:
   ```rust
   let opcode = if self.in_tail_position {
       Opcode::TailCall
   } else {
       Opcode::Call
   };
   ```

**Tests**:
- `TailCall` emitted for: `(fn [x] (f x))`
- `TailCall` emitted for: `(fn [x] (if c (f x) (g x)))`
- `TailCall` emitted for: `(fn [x] (do (println x) (f x)))`
- `TailCall` emitted for: `(fn [x] (let [y 1] (f y)))`
- `Call` emitted for: `(fn [x] (+ 1 (f x)))` (not in tail position)
- `Call` emitted for: `(fn [x] (do (f x) 42))` (not last expression)

**Estimated effort**: 1 context window

---

### Task 1.2.7: Proper Tail Calls - VM Trampoline

**Description**: Restructure the VM interpreter to use a trampoline loop, enabling tail calls without Rust stack growth.

**Files to modify**:
- `crates/lona-kernel/src/vm/interpreter/mod.rs`
- `crates/lona-kernel/src/vm/interpreter/ops_control.rs`
- `crates/lona-kernel/src/vm/frame.rs`

**Requirements**:

1. Define execution result enum:
   ```rust
   enum RunResult {
       /// Normal return with value
       Return(Value),
       /// Tail call - continue with new frame (no Rust recursion)
       TailCall {
           chunk: Arc<Chunk>,
           base: usize,
           arguments: Vec<Value>,
       },
   }
   ```

2. Restructure `run()` as outer trampoline loop:
   ```rust
   pub fn run(&mut self, initial_frame: Frame<'_>) -> Result<Value, Error> {
       let mut current_chunk = Arc::clone(initial_frame.chunk());
       let mut current_base = initial_frame.base();

       loop {
           let frame = Frame::new(&current_chunk, current_base, ...);
           match self.run_inner(&mut frame)? {
               RunResult::Return(value) => return Ok(value),
               RunResult::TailCall { chunk, base, arguments } => {
                   current_chunk = chunk;
                   current_base = base;
                   self.setup_arguments(base, &arguments);
                   // Loop continues - no Rust recursion!
               }
           }
       }
   }
   ```

3. Implement `op_tail_call()`:
   - Collect arguments from registers
   - Look up function and find matching arity body
   - Return `RunResult::TailCall` instead of recursing
   - **Critical**: Do NOT increment `call_depth` for tail calls

4. Frame ownership adjustment:
   - Store `Arc<Chunk>` in the trampoline loop
   - Frame borrows from the Arc
   - This allows frame swapping without lifetime issues

**Tests**:
- Deep tail recursion (10,000+ calls) without stack overflow
- Mutual tail recursion between two functions
- Mix of tail and non-tail calls in same function
- Tail call to different function (not self-recursion)
- Tail call preserves correct return value

**Estimated effort**: 2 context windows

---

### Task 1.2.8: Proper Tail Calls - Integration Tests

**Description**: Comprehensive integration tests for proper tail calls.

**Files to modify**:
- `crates/lona-spec-tests/src/tco.rs` (new)
- `crates/lona-spec-tests/src/lib.rs`

**Requirements**:

1. Self-recursion tests:
   ```clojure
   (defn countdown [n]
     (if (= n 0)
       :done
       (countdown (- n 1))))
   (countdown 100000)  ; Must not stack overflow
   ```

2. Mutual recursion tests:
   ```clojure
   (defn even? [n]
     (if (= n 0) true (odd? (- n 1))))
   (defn odd? [n]
     (if (= n 0) false (even? (- n 1))))
   (even? 100000)  ; Must not stack overflow
   ```

3. Accumulator pattern:
   ```clojure
   (defn sum-to [n acc]
     (if (= n 0)
       acc
       (sum-to (- n 1) (+ acc n))))
   (sum-to 100000 0)
   ```

4. State machine pattern:
   ```clojure
   (defn state-a [n]
     (if (= n 0) :done-a (state-b (- n 1))))
   (defn state-b [n]
     (if (= n 0) :done-b (state-c (- n 1))))
   (defn state-c [n]
     (if (= n 0) :done-c (state-a (- n 1))))
   ```

5. Mixed tail/non-tail:
   ```clojure
   (defn mixed [n]
     (if (= n 0)
       0
       (+ 1 (mixed (- n 1)))))  ; NOT tail - should eventually overflow
   ```

**Estimated effort**: 1 context window

---

## File Change Summary

| File | Changes |
|------|---------|
| `compiler/mod.rs` | Add `in_tail_position` field, `compile_expr_in_context()` |
| `compiler/special_forms.rs` | Propagate tail context through `do`, `if`, `let` |
| `compiler/functions.rs` | Set tail=true for last expression in fn body |
| `compiler/calls.rs` | Emit `TailCall` vs `Call` based on tail context |
| `vm/interpreter/mod.rs` | Add `RunResult` enum, trampoline loop in `run()` |
| `vm/interpreter/ops_control.rs` | Implement `op_tail_call()` returning `RunResult::TailCall` |
| `vm/frame.rs` | Support for Arc-based chunk in trampoline |
| `lona-spec-tests/src/tco.rs` | Integration tests |

---

## References

- [Scheme R5RS - Proper Tail Recursion](https://www.cs.utexas.edu/ftp/garbage/cs345/schintro-v14/schintro_127.html)
- [Clojure mailing list - Why no tail call optimization](https://groups.google.com/g/clojure/c/4bSdsbperNE/m/tXdcmbiv4g0J)
- [Ink language - Tail call elimination in bytecode VMs](https://dotink.co/posts/tce/)
