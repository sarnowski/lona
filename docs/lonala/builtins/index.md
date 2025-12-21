# Built-in Functions

This section documents all functions available in Lonala, organized by implementation:

- **Native (Rust)**: Implemented in Rust, required for hardware access, runtime types, or scheduler integration
- **Lonala**: Implemented in pure Lonala using native primitives

## Design Principle: Minimal Native Functions

Lonala follows the Lisp tradition of building the entire language from minimal primitives. Native functions are only used when:

1. **Hardware access is required** (MMIO, DMA, IRQ)
2. **Runtime type inspection is required** (type predicates)
3. **Core data structure operations** (cons, first, rest on internal representations)
4. **Scheduler/process integration** (spawn, send, irq-wait)
5. **seL4 kernel operations** (domain creation, capabilities)
6. **Runtime introspection** (process-info, domain-info, source)

Everything else — including collection constructors (`list`, `vector`, `hash-map`), sequence operations (`map`, `filter`, `reduce`), and even the REPL — is implemented in Lonala itself.

See [docs/development/minimal-rust.md](../../development/minimal-rust.md) for the authoritative list of native primitives and rationale.

## Native Functions (Rust)

| Category | Description | Status |
|----------|-------------|--------|
| [Type Predicates](type-predicates.md) | `nil?`, `list?`, `fn?`, `type-of`, etc. | *(Planned)* |
| [Collections](collections.md) | `cons`, `first`, `rest`, `conj`, `assoc`, etc. | Partial |
| [Binary Operations](binary.md) | Raw byte buffer operations | *(Planned)* |
| [Symbols](symbols.md) | `symbol`, `gensym` | *(Planned)* |
| [Metadata](metadata.md) | `meta`, `with-meta`, `vary-meta` | *(Planned)* |
| [Sorted Collections](sorted-collections.md) | `sorted-map`, `sorted-set` | *(Planned)* |
| [Hardware Access](hardware.md) | MMIO, DMA, IRQ primitives | *(Planned)* |
| [Time](time.md) | `now-ms`, `send-after` | *(Planned)* |
| [Atoms](atoms.md) (native only) | `atom`, `deref`, `reset!`, `compare-and-set!` | *(Planned)* |
| [I/O](io.md) | `native-print` (bootstrap) | Partial |
| [Processes](processes.md) | `spawn`, `send`, `link`, `monitor`, seL4 ops | *(Planned)* |
| [Regular Expressions](regex.md) | `re-pattern`, `re-find`, etc. (optional) | *(Planned)* |

## Lonala Functions (Built on Primitives)

| Category | Description | Status |
|----------|-------------|--------|
| [Standard Library](stdlib.md) | `map`, `filter`, `reduce`, macros | Partial |
| [Atoms](atoms.md) (Lonala) | `swap!`, `add-watch`, `set-validator!` | *(Planned)* |
| [Collections](collections.md) | `vector`, `hash-map`, `vec`, set operations | *(Planned)* |
| [Error Handling](error-handling.md) | `ok?`, `unwrap!`, `map-ok` | *(Planned)* |

---

