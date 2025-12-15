# ADR-0001: Use Rust for Runtime

## Status

Accepted

## Date

2025-12-15

## Context

Lona is an operating system combining three paradigms:

1. **seL4 microkernel** — formally verified, capability-based foundation
2. **LISP machine philosophy** — runtime introspection, hot-patching, source-first
3. **BEAM/Erlang/OTP concurrency** — lightweight processes, fault tolerance, supervision trees

The Lona runtime (root task) must implement:

- **Lonala compiler** — S-expression parser, macro expansion, bytecode generation
- **Per-process garbage collector** — independent heaps, precise root tracking, incremental collection
- **Green thread scheduler** — millions of lightweight processes, reduction counting for preemption
- **Memory manager** — process heaps, capability-controlled shared regions
- **seL4 interface** — syscalls, capability manipulation, VSpace/CSpace management
- **Dispatch tables** — late binding for hot-patching support
- **Introspection infrastructure** — stack frames, process state, source/provenance tracking
- **Condition/restart system** — non-unwinding error handling with interactive recovery

This is a complex, security-critical component. The choice of implementation language has long-term consequences for safety, maintainability, and alignment with Lona's philosophy.

### Forces

- **Security alignment**: Lona is built on seL4's verified isolation. The runtime should not undermine this with memory-safety bugs.
- **Complexity**: A compiler + GC + scheduler + introspection system is substantial. The implementation language should help manage this complexity.
- **seL4 ecosystem**: Most seL4 userland code is written in C. Library support matters.
- **Long-term maintainability**: The runtime will evolve significantly. Safe refactoring is valuable.
- **Performance**: The runtime is on the critical path. Overhead must be minimal.

## Decision

We will use **Rust** as the implementation language for the Lona runtime.

Specifically:
- The root task and all runtime components will be written in Rust
- seL4 integration will use the `sel4-sys` and `sel4-microkit` crates
- Low-level operations (context switching, some GC internals) will use `unsafe` blocks as needed
- We will not use a C shim layer unless specific seL4 integration issues require it

## Consequences

### Positive

**Memory safety by default**
- Buffer overflows, use-after-free, double-free, and data races are eliminated in safe Rust code
- This extends seL4's verified security guarantees into userland
- A memory bug in the runtime could compromise the entire system; Rust prevents most categories

**Compiler implementation benefits**
- Rust's enums with data model S-expressions naturally (Symbol, List, Number, String variants)
- Pattern matching enables clean, exhaustive AST handling
- The type system catches missing cases at compile time
- String handling is safe and ergonomic (no buffer overruns, no manual length tracking)

**Garbage collector correctness**
- GC invariants (e.g., "this pointer is rooted") can be encoded in the type system
- Root tracking can be enforced structurally rather than by convention
- GC bugs are notoriously hard to debug in C; Rust constrains the bug space

**Capability semantics in types**
- Shared memory access rights (read-only vs read-write) can be encoded as type parameters
- Prevents misuse at compile time, aligning with Lona's capability-based security model

**Introspection metadata**
- Complex metadata structures (source provenance, local variable maps, stack frame info) are naturally expressed with Rust structs and enums
- Derive macros reduce boilerplate

**Refactoring confidence**
- The type system catches errors during refactoring
- Essential for a complex, evolving codebase

**Serialization ecosystem**
- Cross-domain message serialization benefits from `serde`
- Type-safe, performant serialization with derive macros

### Negative

**seL4 ecosystem maturity**
- C has more extensive seL4 library support and documentation
- Some seL4 patterns may require writing or extending Rust bindings
- Fewer examples and tutorials in Rust

**Learning curve**
- Rust's borrow checker requires adjustment for developers new to the language
- Some patterns (especially in GC and scheduler) will require `unsafe` blocks regardless
- Initial development may be slower

**Unsafe code still required**
- Context switching requires inline assembly and stack manipulation
- Some GC internals require unsafe memory operations
- seL4 syscalls are inherently unsafe

**Binary size**
- Rust binaries can be larger than equivalent C (monomorphization, standard library)
- Mitigated by `#![no_std]` and careful feature selection

### Neutral

**Performance**
- Rust and C have equivalent performance characteristics for systems code
- Zero-cost abstractions mean high-level patterns don't add runtime overhead
- `unsafe` blocks allow C-equivalent performance where needed

**Tooling**
- Rust tooling (cargo, clippy, rustfmt) is excellent
- Cross-compilation for ARM64/x86_64 is well-supported
- Debugging is slightly less mature than C (improving rapidly)

## Alternatives Considered

### Alternative 1: C

**Description**: The traditional choice for seL4 userland. All seL4 libraries are native C. Most existing seL4 projects use C.

**Evaluation**:

| Aspect | Assessment |
|--------|------------|
| seL4 integration | Excellent — native support |
| Compiler implementation | Poor — manual AST memory management, painful string handling |
| GC implementation | Poor — easy to introduce memory bugs in GC code itself |
| Memory safety | None — undermines seL4's security guarantees |
| Long-term maintenance | Poor — risky refactoring, subtle bugs accumulate |

**Why not chosen**: Memory unsafety is a fundamental contradiction with Lona's security-focused design. The complexity of a compiler + GC + scheduler in C would accumulate technical debt and bugs. The philosophical misalignment is significant: we're building on a verified kernel but would implement the trusted runtime in an unsafe language.

### Alternative 2: Zig

**Description**: A modern C alternative with better safety patterns, comptime metaprogramming, and excellent C interop.

**Evaluation**:

| Aspect | Assessment |
|--------|------------|
| seL4 integration | Poor — minimal ecosystem support |
| C interop | Excellent — can use C libraries directly |
| Safety | Better than C, but not memory-safe by default |
| Maturity | Concerning — language still evolving, smaller community |

**Why not chosen**: Insufficient seL4 ecosystem support. While Zig offers improvements over C, it doesn't provide Rust's memory safety guarantees. The smaller community and less mature tooling add risk for a foundational component.

### Alternative 3: C++

**Description**: C with better abstractions, RAII, templates, and standard library.

**Evaluation**:

| Aspect | Assessment |
|--------|------------|
| seL4 integration | Acceptable — can use C libraries |
| Abstractions | Better than C — RAII, smart pointers |
| Memory safety | Still fundamentally unsafe |
| Complexity | High — language complexity adds cognitive load |

**Why not chosen**: C++ doesn't provide memory safety guarantees. Its complexity (multiple paradigms, historical baggage) adds cognitive load without commensurate safety benefits. The seL4 ecosystem doesn't favor C++ over C.

### Alternative 4: Ada/SPARK

**Description**: Safety-focused language with optional formal verification capabilities. Some seL4 research projects have used Ada.

**Evaluation**:

| Aspect | Assessment |
|--------|------------|
| Safety | Excellent — strong typing, optional formal verification |
| seL4 integration | Limited — research-level support only |
| Community | Small — harder to find developers, fewer resources |
| Ecosystem | Limited — fewer libraries and tools |

**Why not chosen**: While Ada's safety properties are appealing, the small community and limited ecosystem make it impractical. The learning curve is steep, and seL4 support is research-grade rather than production-ready.

### Alternative 5: Hybrid (C for seL4 layer, Rust for rest)

**Description**: Use C for a thin seL4 interface layer, with the bulk of the runtime in Rust.

**Evaluation**:

| Aspect | Assessment |
|--------|------------|
| seL4 integration | Excellent — native C for syscalls |
| Complexity | Higher — two languages, FFI boundary |
| Safety boundary | Unclear — where does "trusted C" end? |

**Why not chosen**: The `sel4-sys` and `sel4-microkit` Rust crates provide adequate seL4 integration. Adding a C layer increases complexity without significant benefit. If specific integration issues arise, this can be reconsidered.

## Feature-by-Feature Analysis

### Compiler (Parser, AST, Bytecode Generation)

| Aspect | C | Rust |
|--------|---|------|
| AST representation | Structs + unions, manual tagging | Enums with data, pattern matching |
| String handling | Manual (char*, length tracking) | Safe (String, &str, no overflows) |
| Error propagation | Return codes, easy to ignore | Result<T, E>, ? operator |
| Pattern matching | switch + manual destructuring | Native, exhaustive checking |

**Winner**: Rust (significantly better)

### Garbage Collector

| Aspect | C | Rust |
|--------|---|------|
| Memory layout control | Complete | Complete (via unsafe) |
| Root tracking | Manual, error-prone | Can encode in type system |
| Write barriers | Manual | Can be enforced structurally |
| Debugging GC bugs | Extremely difficult | Easier (type constraints) |

**Winner**: Rust (GC correctness is critical)

### Process Scheduler

| Aspect | C | Rust |
|--------|---|------|
| Context switching | Inline asm | Inline asm (naked functions) |
| Process state management | Structs | Structs with derive traits |
| Reduction counting | Simple counter | Same |

**Winner**: Tie (both require similar low-level code)

### Memory Manager (Shared Regions)

| Aspect | C | Rust |
|--------|---|------|
| Capability tracking | Manual bookkeeping | Can encode access rights in types |
| Buffer handling | Pointer arithmetic | Slices with bounds checking |

**Winner**: Rust (type-encoded capabilities align with Lona's model)

### seL4 Integration

| Aspect | C | Rust |
|--------|---|------|
| Library support | Native (libsel4) | sel4-sys, sel4-microkit |
| Documentation | Extensive | Growing |
| Syscall overhead | Zero | Near-zero |

**Winner**: C (slight edge, but Rust is adequate)

### Introspection Infrastructure

| Aspect | C | Rust |
|--------|---|------|
| Metadata structures | Manual layout | Natural with enums/structs |
| Source tracking | Painful string handling | Ergonomic |
| Provenance chains | Error-prone | Type-safe |

**Winner**: Rust (complex metadata benefits from type system)

## Summary

| Criterion | C | Rust | Winner |
|-----------|---|------|--------|
| seL4 integration | ★★★★★ | ★★★★☆ | C (slight) |
| Compiler implementation | ★★☆☆☆ | ★★★★★ | **Rust** |
| Garbage collector | ★★☆☆☆ | ★★★★☆ | **Rust** |
| Scheduler | ★★★★☆ | ★★★★☆ | Tie |
| Memory manager | ★★★☆☆ | ★★★★☆ | **Rust** |
| Introspection | ★★☆☆☆ | ★★★★☆ | **Rust** |
| Memory safety | ★☆☆☆☆ | ★★★★★ | **Rust** |
| Long-term velocity | ★★★☆☆ | ★★★★☆ | **Rust** |
| Ecosystem maturity | ★★★★★ | ★★★★☆ | C (slight) |

Rust wins on the majority of criteria, particularly those most important for a complex, security-critical runtime.

## References

- [seL4 Foundation](https://sel4.systems/)
- [sel4-sys crate](https://crates.io/crates/sel4-sys)
- [seL4 Microkit](https://trustworthy.systems/projects/microkit/)
- [Rust Embedded Book](https://docs.rust-embedded.org/book/)
- Lona Goals Document: `docs/goals.md`
