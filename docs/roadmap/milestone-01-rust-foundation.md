# Milestone 1: Rust Foundation

**Goal**: Complete all Rust code required for the Lona runtime. After this milestone, no new Rust code should be needed.

**Deliverable**: A fully functional VM with processes, GC, domains, condition/restart system, debug infrastructure (Two-Mode Architecture), and all native primitives.

**Phases**: 13 phases covering language features, process model, GC, domain isolation, fault tolerance, native primitives, condition system, introspection, and debug infrastructure.

---

# Phases

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
