## Milestone 1: Rust Foundation

**Goal**: Complete all Rust code required for the Lona runtime. After this milestone, no new Rust code should be needed.

**Deliverable**: A fully functional VM with processes, GC, domains, condition/restart system, debug infrastructure (Two-Mode Architecture), and all native primitives.

**Phases**: 13 phases covering language features, process model, GC, domain isolation, fault tolerance, native primitives, condition system, introspection, and debug infrastructure.

---

## Current State

### Completed (Phases 1-4.4)

| Component | Status | Details |
|-----------|--------|---------|
| Lexer | ✅ Complete | Full S-expression tokenization |
| Parser | ✅ Complete | AST generation, reader macros (`'`, `` ` ``, `~`, `~@`) |
| Bytecode Compiler | ✅ Complete | 25 opcodes, register-based |
| VM Interpreter | ✅ Complete | Bytecode execution, call stack |
| Special Forms | ✅ Complete | `def`, `let`, `if`, `do`, `fn`, `quote`, `syntax-quote`, `defmacro` |
| Macro System | ✅ Complete | Compile-time expansion, introspection |
| Core Values | ✅ Complete | Nil, Bool, Integer, Float, Ratio, Symbol, String, List, Vector, Map |
| Basic Natives | ✅ Complete | `cons`, `first`, `rest`, `list`, `concat` |
| Collection Constructors | ✅ Complete | `vector`, `hash-map`, `vec` (native bootstrap) |
| REPL (Rust) | ✅ Complete | Interactive evaluation (native bootstrap) |
| Rest Arguments | ✅ Complete | `& rest` syntax in functions and macros |

### Missing (Required for Milestone 1)

| Component | Status | Priority |
|-----------|--------|----------|
| Keyword Value Type | ✅ Complete | High |
| Set Value Type | ✅ Complete | High |
| Binary Value Type | ✅ Complete | High |
| Metadata System | ❌ Not Started | High |
| Closures | ❌ Not Started | Critical |
| Multi-Arity Functions | ❌ Not Started | High |
| Destructuring | ❌ Not Started | Critical |
| [Proper Tail Calls](../development/tco.md) | ❌ Not Started | Critical |
| Namespace System | ❌ Not Started | High |
| Process Model | ❌ Not Started | Critical |
| Green Thread Scheduler | ❌ Not Started | Critical |
| Garbage Collection | ❌ Not Started | Critical |
| Domain Isolation | ❌ Not Started | Critical |
| Inter-Domain IPC | ❌ Not Started | Critical |
| MMIO/DMA/IRQ Primitives | ❌ Not Started | Critical |
| All Type Predicates | ⚠️ Partial | High |
| Bitwise Operations | ❌ Not Started | High |
| Atom Primitives | ❌ Not Started | Medium |
| Sorted Collections | ❌ Not Started | Low |
| Condition/Restart System | ❌ Not Started | Critical |
| Introspection System | ❌ Not Started | High |
| Debug Infrastructure | ❌ Not Started | High |

---

## Phases

| Phase | Name | Description |
|-------|------|-------------|
| 1.0 | [Arithmetic Primitives](milestone-01-rust-foundation/00-arithmetic.md) | +, -, *, /, mod, comparisons |
| 1.1 | [Core Value Type Extensions](milestone-01-rust-foundation/01-core-value-types.md) | Keyword, Set, Binary, Metadata |
| 1.2 | [Language Feature Completion](milestone-01-rust-foundation/02-language-features.md) | Closures, destructuring, TCO, pattern matching |
| 1.3 | [Namespace System](milestone-01-rust-foundation/03-namespace-system.md) | Namespaces, Vars, require/use, defnative |
| 1.4 | [Process Model](milestone-01-rust-foundation/04-process-model.md) | Processes, scheduler, spawn/send/receive, dynamic bindings |
| 1.5 | [Garbage Collection](milestone-01-rust-foundation/05-garbage-collection.md) | Tri-color marking, generational GC, per-process isolation |
| 1.6 | [Domain Isolation & IPC](milestone-01-rust-foundation/06-domain-isolation.md) | seL4 VSpace/CSpace, shared memory, capability transfer |
| 1.7 | [Fault Tolerance](milestone-01-rust-foundation/07-fault-tolerance.md) | Links, monitors, exit signals, cross-domain fault tolerance |
| 1.8 | [Native Primitives](milestone-01-rust-foundation/08-native-primitives.md) | Type predicates, bitwise, collections, binary, MMIO/DMA/IRQ |
| 1.9 | [Integration & Spec Tests](milestone-01-rust-foundation/09-integration-tests.md) | Process, domain, GC tests, hot code loading |
| 1.10 | [Condition/Restart System](milestone-01-rust-foundation/10-condition-restart.md) | Conditions, handlers, restarts, REPL integration |
| 1.11 | [Introspection System](milestone-01-rust-foundation/11-introspection.md) | Source retrieval, namespace/process/domain introspection |
| 1.12 | [Debug Infrastructure](milestone-01-rust-foundation/12-debug-infrastructure.md) | Two-Mode Architecture, breakpoints, stepping, debugger REPL |
