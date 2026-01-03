# Lonala Language Specification

Lonala is a LISP dialect for the Lona operating system, running on the seL4 microkernel. It combines Clojure-inspired syntax with BEAM-style concurrency.

---

## Design Philosophy

- **Minimal core**: Exactly 5 special forms; everything else is intrinsics or derived
- **Automatic TCO**: Tail-call optimization is guaranteed; no `recur` needed
- **Pattern matching**: Central to the language, replaces conditionals and destructuring
- **Let it crash**: Tuple returns for errors, supervisor restarts for failures
- **Homoiconic**: Code is data, fully manipulable via macros
- **seL4 foundation**: Capability-based security, hardware-enforced isolation

---

## What Lonala Is NOT

**Lonala is not Clojure.** While inspired by Clojure's syntax and data structures:
- No JVM, no Java interop
- No `recur` (automatic TCO instead)
- No `try`/`catch`/`finally` (tuple returns instead)
- Different collection syntax: `[]` = tuple, `{}` = vector
- No STM (message passing instead)

**Lonala is not Erlang/Elixir.** While inspired by BEAM's process model:
- LISP syntax, not Erlang syntax
- Clojure-style namespaces and vars
- Custom VM on seL4, not BEAM-compatible

---

## Document Overview

| Document | Contents |
|----------|----------|
| [reader.md](reader.md) | Lexical syntax, literals, reader macros |
| [special-forms.md](special-forms.md) | The 5 special forms |
| [data-types.md](data-types.md) | All value types |
| [lona.core.md](lona.core.md) | Core language intrinsics |
| [lona.process.md](lona.process.md) | Process and realm intrinsics |
| [lona.kernel.md](lona.kernel.md) | seL4 kernel intrinsics |
| [lona.io.md](lona.io.md) | Device I/O intrinsics |
| [lona.time.md](lona.time.md) | Time intrinsics |

---

## Intrinsics vs Derived

**Intrinsics** are native functions implemented in the VM. They are documented in the namespace specifications.

**Derived** functions and macros are implemented in Lonala itself. Each namespace document includes an appendix listing expected derived forms. These are not intrinsics.

---

## Type System Overview

```
Lonala Types
├── Scalars
│   ├── nil, true, false
│   ├── Numbers (integer, float, ratio, fixed-width)
│   ├── Character, String
│   └── Symbol, Keyword
├── Collections
│   ├── List (), Tuple [], Vector {}, Map %{}, Set #{}
│   └── Binary #bytes[], Bytebuf
├── Addresses
│   └── paddr, vaddr
└── System Types
    ├── realm-id, pid, ref, notification
    ├── Capabilities (tcb-cap, endpoint-cap, frame-cap, ...)
    └── msg-info, region, dma-buffer, ring
```

See [data-types.md](data-types.md) for complete type documentation.
