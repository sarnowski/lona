# I/O
Basic output primitives.

> **Note**: The REPL and high-level I/O are implemented in Lonala using MMIO primitives. `print` is partially implemented in Rust but not yet exposed as a native function callable from Lonala.

## Functions

| Function | Syntax | Description |
|----------|--------|-------------|
| `print` | `(print args*)` | Print values followed by newline |

## Examples

```clojure
(print "Hello")           ; prints: Hello
(print 1 2 3)             ; prints: 1 2 3
```

## Design Note

Lonala follows the Lonala-first principle: the Rust runtime has its own UART access for panic handlers and early boot diagnostics, but this is NOT exposed to Lonala. Lonala implements device drivers (including UART) using `peek`/`poke` on memory-mapped I/O registers.

See [Hardware Access](hardware.md) for MMIO primitives.

---

