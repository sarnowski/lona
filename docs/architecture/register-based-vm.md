# Register-Based Virtual Machine Design

This document explains the design decisions behind Lonala's bytecode virtual machine, including the choice of a register-based architecture over a stack-based one, instruction format design, and implementation considerations.

---

## Decision: Register-Based VM

Lonala uses a **register-based virtual machine**, similar to BEAM (Erlang), Lua 5.x, and Android's Dalvik. This section documents why this architecture was chosen over a stack-based approach.

### Research Sources

The following sources informed this decision:

- [BEAM VM Primer](https://www.erlang.org/blog/a-brief-beam-primer/) - Erlang's register-based design
- [Lua 5.3 Bytecode Reference](https://the-ravi-programming-language.readthedocs.io/en/latest/lua_bytecode_reference.html) - Instruction encoding
- [The Implementation of Lua 5.0](https://www.lua.org/doc/jucs05.pdf) - Stack→Register migration rationale
- [Virtual Machine Showdown: Stack vs Registers (ACM 2007)](https://dl.acm.org/doi/10.1145/1328195.1328197) - Academic benchmark
- [Register vs Stack VMs in JIT Scenarios (2025)](https://onlinelibrary.wiley.com/doi/full/10.1002/spe.70014) - Recent findings
- [Interpreter Dispatch Techniques](https://realityforge.org/code/virtual-machines/2011/05/19/interpreters.html) - Implementation guidance

### Performance Comparison

| Study | Register Advantage | Notes |
|-------|-------------------|-------|
| ACM 2007 Study | **32.3% faster** on Pentium 4 | 47% fewer VM instructions executed |
| Same study (inline threading) | **15% faster** | Gains narrow with better dispatch |
| 2025 JIT Study | Stack 1.04x faster for recursion (interpreted) | Register 1.21x faster with JIT |
| Lua 4→5 Migration | ~60% fewer instructions | Same code: 49 instructions (stack) vs ~17 (register) |

### Why Register VMs Execute Fewer Instructions

**Stack-based** (e.g., Lua 4.0, JVM): A simple operation `a = b + c` requires:

```
GETLOCAL b    ; push b onto stack
GETLOCAL c    ; push c onto stack
ADD           ; pop b,c; push result
SETLOCAL a    ; pop result into a
```

This is **4 instructions**, each manipulating an implicit operand stack.

**Register-based** (e.g., Lua 5.x, BEAM): The same operation:

```
ADD a, b, c   ; a = b + c
```

This is **1 instruction** with operands encoded explicitly.

### The Trade-off: Instruction Size

| VM Type | Typical Instruction Size | Operand Encoding |
|---------|-------------------------|------------------|
| Stack-based | 1-2 bytes | Implicit (stack position) |
| Register-based | 4 bytes | Explicit (register fields) |

Register instructions are 2-4x larger, but you execute ~50% fewer of them.

### Why Instruction Count Matters More

From interpreter research:

> "The ratio of useful work performed compared to the overhead of traversing the representation is often low. It would not be unexpected for one VM instruction to translate into 1 or 2 native instructions of useful work and **10 or more native instructions to dispatch** to the next VM instruction."

Dispatch overhead dominates interpreter execution time. Reducing instruction count by 50% has a larger impact than saving bytes on instruction encoding.

### Decision Matrix for Lonala

| Factor | Stack-Based | Register-Based | Winner for Lonala |
|--------|-------------|----------------|-------------------|
| Compiler complexity | Lower | Higher | Stack |
| Interpreter speed | ~15-32% slower | Faster | **Register** |
| Instruction debuggability | Simpler trace | Need to track registers | Stack |
| Hot-patching friendliness | Equal | Equal | Tie |
| Future JIT potential | Harder | Easier | **Register** |
| BEAM compatibility | No | Closer to BEAM | **Register** |
| Tail call optimization | Trickier | Natural | **Register** |
| Reduction counting efficiency | More reductions per work | Fewer reductions per work | **Register** |

### Rationale for Choosing Register-Based

1. **BEAM alignment**: Lona aims for BEAM-style semantics. BEAM is register-based.
2. **Performance**: 15-32% interpreter speedup is meaningful for a production OS.
3. **TCO support**: Tail call optimization is natural—overwrite argument registers and jump.
4. **Future JIT path**: Registers map directly to CPU registers.
5. **Reduction counting**: Fewer instructions per unit of work means more accurate scheduling.

---

## Instruction Format

Lonala uses **fixed 32-bit instructions** inspired by Lua 5.x. This provides a balance between simplicity and efficiency.

### Format Types

```
iABC:  [opcode:8][A:8][B:8][C:8]   = 32 bits
iABx:  [opcode:8][A:8][Bx:16]      = 32 bits
iAsBx: [opcode:8][A:8][sBx:16]     = 32 bits (signed, for jumps)
```

**Field meanings:**

| Field | Bits | Range | Purpose |
|-------|------|-------|---------|
| opcode | 8 | 0-255 | Instruction type |
| A | 8 | 0-255 | Destination register |
| B | 8 | 0-255 | Source register or small constant |
| C | 8 | 0-255 | Source register or small constant |
| Bx | 16 | 0-65535 | Extended operand (constant pool index) |
| sBx | 16 | -32768 to 32767 | Signed offset for jumps |

### Register/Constant Encoding

Following Lua's approach, the MSB of B and C fields can indicate whether the operand is a register reference or a constant pool index:

- `B < 256`: Register B
- `B >= 256`: Constant pool index (B - 256)

This allows arithmetic instructions to operate directly on constants without separate load instructions.

### Why 32-bit Fixed Width?

1. **Simplicity**: No variable-length decoding logic
2. **Alignment**: Natural 4-byte alignment for modern CPUs
3. **Sufficient capacity**: 8-bit opcode allows 256 instructions; 8-bit register field allows 256 registers per frame
4. **Proven design**: Lua has used this successfully for 20+ years

---

## Register Model

### Register Types

Lonala uses a simplified version of BEAM's register model:

| Register Type | Symbol | Purpose | Lifetime |
|---------------|--------|---------|----------|
| Argument/Temp | R0-R255 | Arguments, temporaries, return values | Within function |
| Local | L0-L255 | Local variables (in stack frame) | Survives calls |

**Simplification**: Initially, we may use a single register space like Lua (all registers are frame-local). The BEAM-style X/Y split can be added later if needed for performance.

### Calling Convention

Following BEAM and Lua conventions:

1. Arguments passed in registers R0, R1, R2, ... (left to right)
2. Return value placed in R0
3. Caller-save: All registers except R0 are invalid after a call
4. Callee allocates frame for locals beyond arguments

### Stack Frame Layout

```
┌─────────────────────────────────────┐
│ Return address                      │
├─────────────────────────────────────┤
│ Previous frame pointer              │
├─────────────────────────────────────┤
│ R0 (arg 0 / return value)           │
│ R1 (arg 1)                          │
│ R2 (arg 2)                          │
│ ...                                 │
│ Rn (local n)                        │
│ ...                                 │
│ Temporaries                         │
└─────────────────────────────────────┘
```

---

## Instruction Set (Phase 2)

The initial instruction set supports the Phase 2 deliverable: `(print (+ 1 2))` prints `3`.

### Data Movement

| Opcode | Format | Description |
|--------|--------|-------------|
| `Move` | iABC | `R[A] = R[B]` |
| `LoadK` | iABx | `R[A] = K[Bx]` (load from constant pool) |
| `LoadNil` | iABC | `R[A]..R[A+B] = nil` |
| `LoadTrue` | iABC | `R[A] = true` |
| `LoadFalse` | iABC | `R[A] = false` |

### Global Variables

| Opcode | Format | Description |
|--------|--------|-------------|
| `GetGlobal` | iABx | `R[A] = globals[K[Bx]]` |
| `SetGlobal` | iABx | `globals[K[Bx]] = R[A]` |

### Arithmetic

| Opcode | Format | Description |
|--------|--------|-------------|
| `Add` | iABC | `R[A] = RK[B] + RK[C]` |
| `Sub` | iABC | `R[A] = RK[B] - RK[C]` |
| `Mul` | iABC | `R[A] = RK[B] * RK[C]` |
| `Div` | iABC | `R[A] = RK[B] / RK[C]` |
| `Mod` | iABC | `R[A] = RK[B] % RK[C]` |
| `Neg` | iABC | `R[A] = -R[B]` |

`RK[x]` means "register if x < 256, else constant pool at x-256".

### Comparison

| Opcode | Format | Description |
|--------|--------|-------------|
| `Eq` | iABC | `R[A] = RK[B] == RK[C]` |
| `Lt` | iABC | `R[A] = RK[B] < RK[C]` |
| `Le` | iABC | `R[A] = RK[B] <= RK[C]` |
| `Gt` | iABC | `R[A] = RK[B] > RK[C]` |
| `Ge` | iABC | `R[A] = RK[B] >= RK[C]` |
| `Not` | iABC | `R[A] = not R[B]` |

### Control Flow

| Opcode | Format | Description |
|--------|--------|-------------|
| `Jump` | iAsBx | `PC += sBx` |
| `JumpIf` | iAsBx | `if R[A] then PC += sBx` |
| `JumpIfNot` | iAsBx | `if not R[A] then PC += sBx` |

### Function Calls

| Opcode | Format | Description |
|--------|--------|-------------|
| `Call` | iABC | `R[A]..R[A+C-1] = R[A](R[A+1]..R[A+B])` |
| `TailCall` | iABC | `return R[A](R[A+1]..R[A+B])` |
| `Return` | iABC | `return R[A]..R[A+B-1]` |

---

## Constant Pool

Each compiled chunk contains a constant pool storing:

```rust
pub enum Constant {
    Nil,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Symbol(symbol::Id),
}
```

Constants are referenced by index via `LoadK` and `RK` operand encoding.

---

## Chunk Structure

A `Chunk` represents a compiled function or top-level expression:

```rust
pub struct Chunk {
    /// Bytecode instructions.
    code: Vec<u32>,

    /// Constant pool.
    constants: Vec<Constant>,

    /// Number of registers needed.
    max_registers: u8,

    /// Number of parameters (for functions).
    arity: u8,

    /// Source spans for each instruction (debugging).
    spans: Vec<Span>,

    /// Function name for debugging (empty for anonymous/top-level).
    name: String,
}

impl Chunk {
    pub fn new() -> Self;
    pub fn with_name(name: String) -> Self;
    pub fn name(&self) -> &str;
    pub fn set_name(&mut self, name: String);
    pub fn arity(&self) -> u8;
    pub fn set_arity(&mut self, arity: u8);
    pub fn max_registers(&self) -> u8;
    pub fn set_max_registers(&mut self, count: u8);
    pub fn emit(&mut self, instruction: u32, span: Span) -> usize;
    pub fn patch(&mut self, index: usize, instruction: u32);
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn add_constant(&mut self, constant: Constant) -> Result<u16, Error>;
    pub fn get_constant(&self, index: u16) -> Option<&Constant>;
    pub fn code(&self) -> &[u32];
    pub fn constants(&self) -> &[Constant];
    pub fn spans(&self) -> &[Span];
    pub fn span_at(&self, index: usize) -> Option<Span>;
    pub fn disassemble(&self) -> String;
}
```

---

## Dispatch Strategy

### Initial Implementation: Switch Dispatch

Start with portable switch-based dispatch:

```rust
loop {
    let instruction = code[pc];
    pc += 1;

    match opcode(instruction) {
        Opcode::Add => {
            let a = field_a(instruction);
            let b = field_b(instruction);
            let c = field_c(instruction);
            registers[a] = rk(b) + rk(c);
        }
        // ...
    }
}
```

**Advantages:**
- Portable (standard Rust)
- Easy to debug
- Simple implementation

### Future: Computed Goto

Can add computed goto (via `unsafe` and function pointers) later if profiling shows dispatch is a bottleneck. Research shows 15-30% speedup, but adds complexity.

---

## Future Extensions

### Phase 4: Closures

New instructions:

| Opcode | Description |
|--------|-------------|
| `Closure` | Create closure from prototype |
| `GetUpval` | Load upvalue |
| `SetUpval` | Store upvalue |

### Phase 7-8: Processes and Messages

New instructions for BEAM-style concurrency:

| Opcode | Description |
|--------|-------------|
| `Spawn` | Create new process |
| `Send` | Send message to PID |
| `Receive` | Pattern-match on mailbox |
| `Yield` | Cooperative yield point |

### Phase 10: Preemption

Add reduction counting to existing dispatch loop:

```rust
reductions += instruction_cost(opcode);
if reductions >= REDUCTION_LIMIT {
    return ExecuteResult::Yield;
}
```

---

## References

- [BEAM (Erlang virtual machine) - Wikipedia](https://en.wikipedia.org/wiki/BEAM_(Erlang_virtual_machine))
- [Lua 5.0 Implementation Paper](https://www.lua.org/doc/jucs05.pdf)
- [The BEAM Book](https://blog.stenmans.org/theBeamBook/)
- [Interpreter Implementation Choices](https://realityforge.org/code/virtual-machines/2011/05/19/interpreters.html)
- [A Performance Survey on Stack-based and Register-based VMs](https://arxiv.org/abs/1611.00467)
