# Virtual Machine

This document describes the Lona Virtual Machine (Lona VM), the bytecode execution engine that runs Lonala code within realms. It covers the register-based architecture, bytecode format, execution model, and design decisions that enable future optimizations like JIT compilation.

---

## Scope

This document defines the **foundational VM architecture**. The implementation is incremental:

- **Initial types**: nil, bool, int, string, symbol, pair (see [Value Tags](#value-tags))
- **Future types**: Added as needed (keywords, floats, collections, capabilities, etc.)
- **Future features**: Pattern matching, closures, message passing, GC — built on these foundations

The architecture is designed to support future features without breaking changes. The implementation source code is the authoritative reference for what is currently implemented.

---

## Overview

The Lona VM is a register-based bytecode interpreter embedded in every realm. It executes compiled Lonala code, schedules lightweight processes, and manages per-process memory.

| Property | Description |
|----------|-------------|
| **Architecture** | Register-based (not stack-based) |
| **Instruction size** | Fixed 32-bit (4 bytes) |
| **Value size** | 64-bit tagged values |
| **Registers** | X registers (temporaries), Y registers (locals) |
| **Dispatch** | Direct threading (function pointers) |
| **Scheduling** | Reduction-based cooperative preemption |

### Why a Register-Based VM?

Modern high-performance VMs (BEAM, Lua 5.0+, Dalvik/ART) use register-based architectures rather than stack-based ones. The Lona VM follows this approach for several reasons:

| Aspect | Stack-Based | Register-Based |
|--------|-------------|----------------|
| **Instructions executed** | More (push/pop overhead) | ~46% fewer |
| **Code density** | Smaller bytecode | ~26% larger bytecode |
| **Dispatch overhead** | Higher (more instructions) | Lower (fewer dispatches) |
| **JIT compilation** | Harder to optimize | Maps naturally to hardware registers |
| **Compiler complexity** | Simple | Moderate (register allocation) |

**Key insight**: The reduction in dispatch overhead outweighs the increase in bytecode size. Each instruction fetch, decode, and dispatch has fixed cost; register machines amortize this cost over more work per instruction.

---

## Design Philosophy

### Principles

1. **Simplicity over cleverness**: Clear, understandable design that can be reasoned about
2. **Performance without sacrificing correctness**: Optimize the common case, but never at the cost of correctness
3. **JIT-friendly from the start**: Design decisions should not preclude future JIT compilation
4. **BEAM-inspired scheduling**: Reduction counting for fair, preemptive-cooperative scheduling
5. **Fixed-width instructions**: Predictable memory access patterns, branch-predictor friendly

### What We Take From Each System

| Source | What We Adopt |
|--------|---------------|
| **BEAM** | X/Y register split, reduction counting, BIF (intrinsic) system |
| **Lua 5.0+** | Fixed 32-bit instruction format, compact operand encoding |
| **JVM/Dalvik** | Constant pool design, method structure |
| **CPython** | Simple dispatch loop as baseline, computed gotos for optimization |

---

## Value Representation

All values in the VM are represented as tagged 64-bit words. This enables efficient register storage and comparison.

### Tagged Value Layout

```
64-bit Tagged Value
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  Immediate values (tag in low bits):                                │
│  ┌───────────────────────────────────────────────────────┬────────┐ │
│  │  Payload (60 bits)                                    │ Tag(4) │ │
│  └───────────────────────────────────────────────────────┴────────┘ │
│                                                                     │
│  Pointer values (tag in low bits, pointer in high bits):            │
│  ┌───────────────────────────────────────────────────────┬────────┐ │
│  │  Heap Pointer (60 bits, 16-byte aligned)              │ Tag(4) │ │
│  └───────────────────────────────────────────────────────┴────────┘ │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Value Tags

The 4-bit tag space supports up to 16 primary types:

| Tag | Type | Description |
|-----|------|-------------|
| `0x0` | Nil | The nil value (immediate) |
| `0x1` | Bool | Boolean (immediate, payload: 0=false, 1=true) |
| `0x2` | Int | Small integer (immediate, 60-bit signed) |
| `0x3` | String | UTF-8 string (pointer to heap) |
| `0x4` | Symbol | Interned symbol (pointer to heap) |
| `0x5` | Pair | Cons cell for lists (pointer to heap) |
| `0x6` | Keyword | Interned keyword (pointer to heap) |
| `0x7` | Tuple | Fixed-size tuple (pointer to heap) |
| `0x8` | Map | Association list map (pointer to heap) |
| `0x9` | CompiledFn | Pure function without captures (pointer to heap) |
| `0xA` | Closure | Function with captured values (pointer to heap) |
| `0xB` | NativeFn | Intrinsic ID (immediate, 16-bit payload) |
| `0xC` | Var | VarSlot reference (pointer to code region) |
| `0xD` | Namespace | Namespace reference (pointer to code region) |
| `0xE` | Unbound | Sentinel for uninitialized vars (immediate) |
| `0xF` | Reserved | Future types |

**Callable types**:
- `CompiledFn` points to a pure function (no captures) - result of `(fn* [x] ...)`
- `Closure` points to a function paired with captured values - result of `(fn* [x] (fn* [y] (+ x y)))`
- `NativeFn` is an immediate value containing an intrinsic dispatch ID (0-65535)
- All three return `true` for the `fn?` predicate

**Note**: The complete type system includes additional types (floats, vectors, sets, capabilities, etc.) documented in `docs/lonala/data-types.md`. Types are added to the VM as needed. The implementation source code (`crates/lona-vm/src/value/`) is the authoritative reference for currently supported types.

### Small Integer Optimization

Most integers in typical programs fit in 60 bits. These are stored inline without heap allocation:

```
Small Integer (inline, no allocation)
┌─────────────────────────────────────────────────────────────────────┐
│  Range: -2^59 to 2^59 - 1  (±576 quadrillion)                       │
│                                                                     │
│  ┌───────────────────────────────────────────────────────┬────────┐ │
│  │  Signed Integer Value (60 bits)                       │ 0x2    │ │
│  └───────────────────────────────────────────────────────┴────────┘ │
│                                                                     │
│  Operations: Add, subtract, multiply check for overflow.            │
│  On overflow: Promote to BigInt (heap-allocated).                   │
└─────────────────────────────────────────────────────────────────────┘
```

### Pointer Alignment

Heap-allocated values are aligned to 16 bytes. This provides 4 bits for the tag in the pointer itself, eliminating the need for separate tag storage:

```
Pointer Value (16-byte aligned)
┌─────────────────────────────────────────────────────────────────────┐
│  Memory address: 0x...........0  (low 4 bits always zero)           │
│                                                                     │
│  Tagged pointer: address | tag                                      │
│                                                                     │
│  To get address: value & ~0xF                                       │
│  To get tag:     value & 0xF                                        │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Register Architecture

The VM uses two classes of registers, inspired by BEAM's design.

### X Registers (Temporaries)

X registers are general-purpose registers used for:
- Function arguments (X0, X1, X2, ...)
- Return values (X0)
- Intermediate computation results
- Temporary storage during expression evaluation

```
X REGISTERS (Temporaries)
════════════════════════════════════════════════════════════════════════

┌────┬────┬────┬────┬────┬────┬────┬────┬─────────┐
│ X0 │ X1 │ X2 │ X3 │ X4 │ X5 │ X6 │ X7 │ ... X255│
└────┴────┴────┴────┴────┴────┴────┴────┴─────────┘
  │    │    │
  │    │    └── Third argument / temp
  │    └─────── Second argument / temp
  └──────────── First argument / return value

Properties:
- NOT preserved across function calls
- Caller-save semantics
- 256 registers available (8-bit addressing)
- Stored in process state, not on stack
```

### Y Registers (Locals)

Y registers are stack-frame-local variables that persist across function calls:

```
Y REGISTERS (Stack Frame Locals)
════════════════════════════════════════════════════════════════════════

Stack frame for function with 3 locals:
┌─────────────────────────────────────────────────────────────────────┐
│  Return Address (continuation pointer)                              │
├─────────────────────────────────────────────────────────────────────┤
│  Y0: Local variable 0                                               │
├─────────────────────────────────────────────────────────────────────┤
│  Y1: Local variable 1                                               │
├─────────────────────────────────────────────────────────────────────┤
│  Y2: Local variable 2                                               │
└─────────────────────────────────────────────────────────────────────┘

Properties:
- Preserved across function calls (callee-save semantics)
- Allocated per stack frame
- Y register count known at compile time for each function
- Addressed relative to frame pointer
```

### Register Allocation Strategy

The compiler allocates registers as follows:

1. **Arguments**: Passed in X0, X1, X2, ... (up to limit, then spill)
2. **Return value**: Always in X0
3. **Temporaries**: Allocated from available X registers during expression evaluation
4. **Local variables**: Assigned to Y registers if they survive function calls
5. **Let bindings**: Use X registers for short-lived bindings, Y for long-lived

```
EXAMPLE: Compiling (let [a (+ 1 2)] (foo a a))
════════════════════════════════════════════════════════════════════════

; 'a' is used twice after a call, so it needs Y register
ALLOCATE 1          ; Reserve Y0 for 'a'
ADD X0, 1, 2        ; X0 = 1 + 2
MOVE Y0, X0         ; Y0 = X0 (save 'a' in local)
MOVE X0, Y0         ; First arg = a
MOVE X1, Y0         ; Second arg = a
CALL foo/2          ; Call foo with 2 args
DEALLOCATE 1        ; Release stack frame
RETURN              ; Return X0
```

---

## Bytecode Format

Instructions are fixed 32-bit words with several encoding formats.

### Instruction Encoding

```
32-BIT INSTRUCTION FORMATS
════════════════════════════════════════════════════════════════════════

Format A: Three operands (arithmetic, comparisons)
┌────────┬────────┬─────────┬─────────┐
│ Opcode │   A    │    B    │    C    │
│ 6 bits │ 8 bits │  9 bits │  9 bits │
└────────┴────────┴─────────┴─────────┘

Format B: Two operands + unsigned immediate (loads, jumps)
┌────────┬────────┬───────────────────┐
│ Opcode │   A    │        Bx         │
│ 6 bits │ 8 bits │      18 bits      │
└────────┴────────┴───────────────────┘

Format C: Two operands + signed immediate (relative jumps)
┌────────┬────────┬───────────────────┐
│ Opcode │   A    │        sBx        │
│ 6 bits │ 8 bits │  18 bits (signed) │
└────────┴────────┴───────────────────┘

Format D: One operand + large immediate (extended)
┌────────┬────────────────────────────┐
│ Opcode │           Ax               │
│ 6 bits │         26 bits            │
└────────┴────────────────────────────┘
```

### Operand Encoding

The B and C fields use 9 bits each, allowing:
- **Register reference** (bit 8 = 0): Register index 0-255
- **Constant reference** (bit 8 = 1): Constant pool index 0-255

```
OPERAND ENCODING (9 bits)
════════════════════════════════════════════════════════════════════════

┌───────────┬──────────────────────────────────────────────────────────┐
│ Bit 8     │ Interpretation                                           │
├───────────┼──────────────────────────────────────────────────────────┤
│ 0         │ Register: bits 0-7 = register index (0-255)              │
│ 1         │ Constant: bits 0-7 = constant pool index (0-255)         │
└───────────┴──────────────────────────────────────────────────────────┘

This encoding is called RK (Register or Konstant) in Lua terminology.

Example:
  ADD X0, X1, K5    ; X0 = X1 + constants[5]
  Encodes as:
  - Opcode: ADD
  - A: 0 (X0)
  - B: 1 (X1, bit 8 = 0)
  - C: 261 (5 + 256, bit 8 = 1)
```

### X vs Y Register Encoding

The VM uses **context-dependent register class encoding** (like BEAM). The opcode determines which register class is used, not the operand encoding:

```
CONTEXT-DEPENDENT REGISTER CLASSES
════════════════════════════════════════════════════════════════════════

X registers (temporaries):
  - Used by: arithmetic, comparison, function arguments, return values
  - Instructions: ADD, SUB, MUL, MOVE, CALL, RETURN, INTRINSIC, etc.
  - All general computation uses X registers

Y registers (locals):
  - Used by: stack frame allocation, local variable storage
  - Instructions: ALLOCATE, DEALLOCATE, and dedicated move instructions

Transfer between classes:
  MOVE_XY   A, B      Y(A) := X(B)    ; Save X to Y (before call)
  MOVE_YX   A, B      X(A) := Y(B)    ; Restore Y to X (after call)

This approach:
- Matches BEAM's semantic model (X for temps, Y for preserved locals)
- No wasted bits in instruction encoding
- Semantically clear: arithmetic never touches Y directly
- Full 256 registers available in each class
```

**Why not encode register class in the operand?** Encoding both class and index in 8 bits would limit each class to 128 registers. The context-dependent approach provides 256 X registers (ample for expression evaluation) and 256 Y registers (ample for local variables) without encoding overhead.

### Why Fixed 32-bit Instructions?

| Benefit | Description |
|---------|-------------|
| **Predictable fetch** | Always read 4 bytes, no length decoding |
| **Aligned access** | No unaligned memory reads |
| **Branch prediction** | Fixed stride aids CPU prediction |
| **JIT friendly** | Easy to map to native instructions |
| **Simplicity** | No variable-length encoding complexity |

The trade-off is slightly larger bytecode (~26% vs variable-length), but the performance benefits from predictable memory access patterns outweigh this cost.

---

## Instruction Set

### Instruction Categories

| Category | Purpose | Examples |
|----------|---------|----------|
| **Data Movement** | Move values between registers/constants | `MOVE`, `LOADK`, `LOADNIL` |
| **Arithmetic** | Integer and float operations | `ADD`, `SUB`, `MUL`, `DIV`, `MOD` |
| **Comparison** | Value comparison, type checking | `EQ`, `LT`, `LE`, `TYPE` |
| **Logic** | Boolean operations | `NOT`, `AND`, `OR` |
| **Control Flow** | Branching and loops | `JMP`, `JMPIF`, `JMPIFNOT` |
| **Function Calls** | Calling and returning | `CALL`, `TAILCALL`, `RETURN` |
| **Intrinsics** | Built-in function dispatch | `INTRINSIC` |
| **Stack Frame** | Local variable management | `ALLOCATE`, `DEALLOCATE` |
| **Collections** | Tuple/vector/map operations | `TUPLE`, `GET`, `PUT` |
| **Process** | Spawn, send, receive | `SPAWN`, `SEND`, `RECV` |

### Core Instructions

```
CORE INSTRUCTION SET
════════════════════════════════════════════════════════════════════════

Data Movement:
  MOVE      A, B         R(A) := R(B)
  LOADK     A, Bx        R(A) := K(Bx)
  LOADNIL   A, B         R(A), R(A+1), ..., R(A+B) := nil
  LOADBOOL  A, B, C      R(A) := (B != 0); if C, skip next

Arithmetic (all support RK operands):
  ADD       A, B, C      R(A) := RK(B) + RK(C)
  SUB       A, B, C      R(A) := RK(B) - RK(C)
  MUL       A, B, C      R(A) := RK(B) * RK(C)
  DIV       A, B, C      R(A) := RK(B) / RK(C)
  MOD       A, B, C      R(A) := RK(B) % RK(C)
  NEG       A, B         R(A) := -R(B)

Comparison (result in condition flag, used by following JMP):
  EQ        A, B, C      if (RK(B) == RK(C)) != A then skip next
  LT        A, B, C      if (RK(B) <  RK(C)) != A then skip next
  LE        A, B, C      if (RK(B) <= RK(C)) != A then skip next

Control Flow:
  JMP       sBx          PC += sBx
  JMPIF     A, sBx       if R(A) is truthy then PC += sBx
  JMPIFNOT  A, sBx       if R(A) is falsy then PC += sBx

Function Calls:
  CALL      A, B, C      R(A), ..., R(A+C-2) := R(A)(R(A+1), ..., R(A+B-1))
  TAILCALL  A, B         return R(A)(R(A+1), ..., R(A+B-1))
  RETURN    A, B         return R(A), ..., R(A+B-2)

Stack Frame:
  ALLOCATE  A            Allocate A stack slots for Y registers
  DEALLOCATE A           Deallocate A stack slots

Y Register Transfer:
  MOVE_XY   A, B         Y(A) := X(B)    ; Save to local
  MOVE_YX   A, B         X(A) := Y(B)    ; Restore from local

Intrinsics:
  INTRINSIC A, B         X0 := intrinsic(A)(X1, ..., X(B))
```

### Instruction Execution Cost

Each instruction has an associated reduction cost for scheduling fairness:

| Instruction Type | Reductions | Rationale |
|------------------|------------|-----------|
| Simple (MOVE, LOADK) | 1 | Basic data movement |
| Arithmetic | 1 | Single operation |
| Comparison | 1 | Single comparison |
| Jump | 1 | Branch |
| Function call | 1 + arg_count | Setup overhead |
| Intrinsic | 1-10+ | Depends on intrinsic |
| Collection ops | 1-5 | Depends on size |
| Process ops | 10-100 | I/O and scheduling |

---

## Constant Pool

Each compiled function (chunk) has a constant pool storing values that don't fit in instruction immediates.

### Constant Pool Contents

| Entry Type | Description |
|------------|-------------|
| **Large integers** | Integers > 60 bits |
| **Floats** | All floating-point numbers |
| **Strings** | String literals |
| **Symbols** | Symbol literals (keywords, identifiers) |
| **Function prototypes** | Nested function definitions |

### Constant Pool Structure

```
CONSTANT POOL
════════════════════════════════════════════════════════════════════════

Chunk {
    code: [u32; N],           // Instruction array
    constants: [Value; M],    // Constant pool
    ...
}

Accessing constant K5:
  LOADK X0, 5       ; X0 = constants[5]

Constants are pre-allocated on the heap when the chunk is loaded.
References from instructions are indices into this array.
```

### String Interning

Identical string literals within a chunk share the same constant pool entry. Symbol literals are globally interned across the entire realm for fast equality comparison.

---

## Execution Model

### The Dispatch Loop

The VM executes instructions in a tight loop:

```
DISPATCH LOOP (Conceptual)
════════════════════════════════════════════════════════════════════════

fn execute(process: &mut Process) -> RunResult {
    loop {
        // Check reduction budget
        if process.reductions == 0 {
            return RunResult::Yielded;
        }

        // Fetch instruction
        let instruction = process.chunk.code[process.pc];
        process.pc += 1;

        // Decode opcode
        let opcode = instruction >> 26;

        // Dispatch to handler
        match opcode {
            OP_MOVE => {
                let a = decode_a(instruction);
                let b = decode_b(instruction);
                process.reg[a] = process.reg[b];
                process.reductions -= 1;
            }
            OP_ADD => {
                let a = decode_a(instruction);
                let b = decode_rk(instruction, process);
                let c = decode_rk_c(instruction, process);
                process.reg[a] = add(b, c)?;
                process.reductions -= 1;
            }
            OP_RETURN => {
                return RunResult::Completed(process.reg[0]);
            }
            // ... other opcodes
        }
    }
}
```

### Dispatch Optimization: Direct Threading

For better performance, the VM uses direct threading where each opcode is replaced with a pointer to its handler:

```
DIRECT THREADING
════════════════════════════════════════════════════════════════════════

Traditional switch dispatch:
  fetch → decode → switch → execute → loop back

Direct threading:
  fetch → jump to handler → execute → jump to next handler

Implementation (using computed goto):

    // Handler table
    static HANDLERS: [fn(); 64] = [handle_move, handle_add, ...];

    // Dispatch
    let handler = HANDLERS[opcode];
    handler();  // Handler ends with: goto *HANDLERS[next_opcode]

Benefits:
- Eliminates switch overhead
- Better branch prediction
- ~15-25% faster than switch dispatch
```

### Reduction Counting

Every instruction decrements a reduction counter. When the counter reaches zero, the process yields:

```
REDUCTION COUNTING
════════════════════════════════════════════════════════════════════════

const MAX_REDUCTIONS: u32 = 4000;  // ~1ms time slice

Process execution:
1. Set reductions = MAX_REDUCTIONS
2. Execute instructions, decrementing reductions
3. When reductions == 0, yield to scheduler
4. Scheduler picks next process
5. Repeat

This ensures:
- Fair scheduling among processes
- No process can monopolize CPU
- Bounded latency for other processes
- Works with cooperative yielding (receive, etc.)
```

---

## Intrinsics (Built-in Functions)

Intrinsics are built-in operations implemented in the VM runtime, not in bytecode.

### Intrinsic Design

```
INTRINSIC SYSTEM
════════════════════════════════════════════════════════════════════════

Intrinsics provide:
- Performance-critical operations (arithmetic on heap values)
- Operations that can't be expressed in Lonala (I/O, process ops)
- Type-specific optimized paths

Intrinsic invocation uses a fixed calling convention:
  INTRINSIC A, B       ; X0 := intrinsic(A)(X1, X2, ..., X(B))

Where:
- A = intrinsic ID (0-255)
- B = argument count

Arguments: X1, X2, ..., X(B)
Result: always in X0

This fixed convention:
- Matches function call semantics (args in X regs, result in X0)
- Simplifies dispatch (no destination register to decode)
- Compiler places args in X1+, result is always in X0
```

### Intrinsic Categories

| Category | Intrinsics | Description |
|----------|------------|-------------|
| **Arithmetic** | `+`, `-`, `*`, `/`, `mod` | Numeric operations |
| **Comparison** | `=`, `<`, `>`, `<=`, `>=` | Value comparison |
| **Type** | `nil?`, `int?`, `str?`, `type` | Type predicates |
| **String** | `str`, `str-len`, `str-concat` | String operations |
| **Collection** | `get`, `put`, `count`, `conj` | Collection ops |
| **Process** | `spawn`, `send`, `self`, `exit` | Process management |
| **I/O** | `read`, `write`, `open`, `close` | Input/output |

### Intrinsic Dispatch

Intrinsics are dispatched through a function pointer table for O(1) lookup:

```
INTRINSIC DISPATCH
════════════════════════════════════════════════════════════════════════

type IntrinsicFn = fn(&mut Process, argc: u8) -> Result<Value, Error>;

static INTRINSICS: [IntrinsicFn; 256] = [
    intrinsic_add,      // ID 0
    intrinsic_sub,      // ID 1
    intrinsic_mul,      // ID 2
    // ...
];

fn dispatch_intrinsic(process: &mut Process, id: u8, argc: u8) -> Result<(), Error> {
    let handler = INTRINSICS[id as usize];
    // Args are in X1, X2, ..., X(argc) - intrinsic reads them directly
    let result = handler(process, argc)?;
    // Result always goes to X0 (fixed calling convention)
    process.x_regs[0] = result;
    Ok(())
}

Example compilation of (+ a b):
  MOVE X1, <a>           ; First arg
  MOVE X2, <b>           ; Second arg
  INTRINSIC ADD, 2       ; X0 = intrinsic_add(X1, X2)
  ; Result now in X0
```

---

## Stack Frames and Calling Convention

### Stack Layout

```
STACK FRAME LAYOUT
════════════════════════════════════════════════════════════════════════

Stack grows downward (high to low addresses)

┌─────────────────────────────────────────────────────────────────────┐
│                          Caller's Frame                              │
├─────────────────────────────────────────────────────────────────────┤
│  Return Address (PC to resume after call)                           │
├─────────────────────────────────────────────────────────────────────┤
│  Previous Frame Pointer                                             │
├─────────────────────────────────────────────────────────────────────┤
│  Y0 (first local)                                                   │
├─────────────────────────────────────────────────────────────────────┤
│  Y1 (second local)                                                  │
├─────────────────────────────────────────────────────────────────────┤
│  Y2 (third local)                                                   │
├─────────────────────────────────────────────────────────────────────┤
│  ... more locals ...                                                │
├─────────────────────────────────────────────────────────────────────┤
│  ← Frame Pointer (FP) points here                                   │
│                                                                     │
│  ← Stack Pointer (SP) points to next free slot                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Calling Convention

```
CALLING CONVENTION
════════════════════════════════════════════════════════════════════════

1. CALLER prepares call:
   - Place arguments in X0, X1, X2, ...
   - Execute CALL instruction

2. CALL instruction:
   - Push return address
   - Push frame pointer
   - Set new frame pointer
   - Jump to callee

3. CALLEE entry:
   - ALLOCATE N (reserve N slots for Y registers)
   - Execute function body

4. CALLEE exit:
   - Place return value in X0
   - DEALLOCATE N (release Y register slots)
   - RETURN (restore FP, jump to return address)

5. CALLER resumes:
   - Return value is in X0
   - Continue execution
```

### Tail Call Optimization

Tail calls reuse the current stack frame:

```
TAIL CALL OPTIMIZATION
════════════════════════════════════════════════════════════════════════

Regular call:
  CALL f    ; Pushes new frame, call f, returns here
  RETURN    ; Returns to our caller

Tail call:
  TAILCALL f  ; Reuses current frame, jumps to f
              ; f's RETURN goes directly to our caller

The TAILCALL instruction:
1. Deallocates current frame
2. Moves arguments to correct positions
3. Jumps to target function
4. Target's RETURN returns to original caller

This enables efficient recursion without stack growth.
```

---

## Memory Model

For the complete process memory model including garbage collection, heap fragments, and per-worker allocators, see [Process Model](process-model.md).

### Process Memory Overview

Each process has two memory blocks following the BEAM model:

```
PROCESS MEMORY MODEL
════════════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────────┐
│  X Registers (in process struct, not on heap)                       │
│  ┌────┬────┬────┬────┬────┬────────────────┐                        │
│  │ X0 │ X1 │ X2 │ X3 │ X4 │ ... X255       │                        │
│  └────┴────┴────┴────┴────┴────────────────┘                        │
├─────────────────────────────────────────────────────────────────────┤
│  Young Heap (stack + young objects, single contiguous block)        │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  STACK (grows down)    │  FREE  │  YOUNG HEAP (grows up)    │    │
│  │  [Frame2][Frame1][F0]◄─┼────────┼─►[string][tuple][cons]    │    │
│  │          stop          │        │        htop               │    │
│  └─────────────────────────────────────────────────────────────┘    │
│  Out of memory when htop >= stop → triggers Minor GC                │
├─────────────────────────────────────────────────────────────────────┤
│  Old Heap (promoted objects, separate contiguous block)             │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  [promoted][promoted][promoted]        │       FREE         │    │
│  │  Objects that survived Minor GC        │◄─ old_htop         │    │
│  └─────────────────────────────────────────────────────────────┘    │
│  Collected only during Major GC (fullsweep)                         │
├─────────────────────────────────────────────────────────────────────┤
│  Mailbox (MPSC queue, messages on receiver's heap)                  │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  [Msg1] → [Msg2] → [Msg3] → nil                             │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

### Heap Allocation

The young heap uses a bump allocator. Allocation is O(1):

```
HEAP ALLOCATION (Young Heap)
════════════════════════════════════════════════════════════════════════

fn alloc(heap, size, align) -> Vaddr {
    let new_htop = align_up(heap.htop + size, align);
    if new_htop >= heap.stop {
        trigger_minor_gc();
        // Retry after GC - young heap is now empty
        new_htop = align_up(heap.htop + size, align);
    }
    let ptr = heap.htop;
    heap.htop = new_htop;
    ptr
}

Generational GC:
- Minor GC: Promotes live young objects to old heap, resets young heap
- Major GC: Collects both heaps, compacts all live data
```

---

## Compiled Code Structure

### Chunk (Compiled Function)

```
CHUNK STRUCTURE
════════════════════════════════════════════════════════════════════════

struct Chunk {
    // Metadata
    name: Symbol,           // Function name (for debugging)
    arity: u8,              // Number of parameters
    num_locals: u8,         // Number of Y registers needed
    is_variadic: bool,      // Accepts variable arguments?

    // Code
    code: Vec<u32>,         // Instruction array
    constants: Vec<Value>,  // Constant pool

    // Debug info (optional)
    line_numbers: Vec<u32>, // Source line for each instruction
    local_names: Vec<Symbol>, // Names of local variables
}
```

### Module (Compiled Namespace)

```
MODULE STRUCTURE
════════════════════════════════════════════════════════════════════════

struct Module {
    name: Symbol,                  // Namespace name (e.g., 'lona.core)
    chunks: Vec<Chunk>,            // Compiled functions
    vars: HashMap<Symbol, Value>,  // Module-level var bindings
    imports: Vec<Symbol>,          // Required namespaces
}
```

---

## Future Considerations

### JIT Compilation Path

The VM architecture supports future JIT compilation:

```
JIT COMPILATION PATH
════════════════════════════════════════════════════════════════════════

Tiered compilation strategy:

Tier 0: Interpreter
- All code starts here
- Profile execution (hot paths, type information)
- Low startup cost

Tier 1: Baseline JIT
- Compile hot functions to native code
- No optimization, fast compilation
- ~5-10x interpreter speedup

Tier 2: Optimizing JIT
- Compile very hot functions with optimizations
- Type specialization, inlining, dead code elimination
- ~50-100x interpreter speedup

Register-based design benefits:
- X/Y registers map to hardware registers
- No stack manipulation overhead
- Straightforward instruction selection
```

### Speculative Optimization

Type information gathered during interpretation enables speculative optimization:

```
SPECULATIVE OPTIMIZATION
════════════════════════════════════════════════════════════════════════

Type profiling during interpretation:
- Track types seen for each operation
- Record branch taken frequencies
- Identify monomorphic call sites

Optimization opportunities:
- Inline small integer arithmetic (skip overflow check)
- Specialize polymorphic calls to monomorphic
- Eliminate redundant type checks
- Inline frequently called functions

Deoptimization:
- Guard fails → return to interpreter
- Recompile with updated type information
```

### On-Stack Replacement (OSR)

OSR enables switching from interpreted to compiled code mid-execution:

```
ON-STACK REPLACEMENT
════════════════════════════════════════════════════════════════════════

Hot loop detection:
- Count loop iterations
- When threshold exceeded, compile loop

OSR entry:
1. Compile loop with current variable values
2. At next loop iteration check:
   - If compiled version ready, transfer execution
   - Map interpreter state to compiled state
   - Continue in compiled code

OSR exit (deoptimization):
1. Guard fails in compiled code
2. Map compiled state back to interpreter state
3. Continue in interpreter
```

---

## Summary

| Aspect | Design Choice | Rationale |
|--------|---------------|-----------|
| **Architecture** | Register-based | Fewer instructions, JIT-friendly |
| **Instruction size** | Fixed 32-bit | Predictable fetch, aligned access |
| **Value representation** | Tagged 64-bit | Inline small integers, fast type checks |
| **Register classes** | X (temp) + Y (local) | Efficient calling convention |
| **Dispatch** | Direct threading | Low overhead, good branch prediction |
| **Scheduling** | Reduction counting | Fair, bounded latency |
| **Intrinsics** | Function pointer table | O(1) dispatch, extensible |
| **Memory** | Per-process heap | Independent GC, no shared state |

The Lona VM is designed to be simple enough to implement correctly, yet structured to allow future optimizations including JIT compilation. Every design decision considers both immediate implementation needs and long-term performance goals.
