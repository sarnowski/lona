# Appendix C: Differences from Clojure
Lonala is inspired by Clojure but differs in several ways:

| Feature | Clojure | Lonala |
|---------|---------|--------|
| **Runtime** | JVM | seL4 / custom VM |
| **Concurrency** | STM + atoms + agents | Erlang-style processes |
| **Interop** | Java interop | No FFI; systems programming primitives |
| **Lazy sequences** | Default | *(Planned)*, explicit |
| **Namespaces** | First-class | *(Planned)* (Phase 6) |
| **Metadata** | Pervasive | *(Planned)* |
| **Protocols** | Supported | *(Planned)* |
| **Transducers** | Supported | *(Planned)* |
| **Keywords** | Full support | *(Planned)* — currently parsed but not represented as values |
| **Regular expressions** | `#"pattern"` | *(Planned)* |
| **Sets** | `#{1 2 3}` | *(Planned)* |

## Key Philosophical Differences

### Concurrency Model

Clojure uses Software Transactional Memory (STM) with atoms, refs, and agents for managing shared state across threads. Lonala uses Erlang-style isolated processes that communicate exclusively through message passing.

### Error Handling

Clojure uses Java exceptions. Lonala uses tagged result tuples (`{:ok value}` / `{:error reason}`) following the Erlang/Elixir convention.

### Systems Programming

Lonala includes primitives for systems programming that Clojure lacks:
- Memory-mapped I/O (`peek-u32`, `poke-u32`)
- DMA buffer allocation
- Interrupt handling
- Capability-based security (via seL4)

### Platform

Clojure runs on the JVM (or JavaScript via ClojureScript). Lonala runs directly on seL4 with no operating system layer between the language runtime and the microkernel.

---

