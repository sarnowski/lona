# Lonala Language Specification

Lonala is the programming language for the Lona operating system. It combines Clojure's elegant syntax and immutable data structures with Erlang's actor-based concurrency model, designed to run on seL4's capability-based microkernel.

## Table of Contents

### Core Language

1. [Introduction](introduction.md) - Overview, design influences, quick start
2. [Lexical Structure](lexical-structure.md) - Character set, whitespace, comments, tokens
3. [Data Types](data-types.md) - Type hierarchy, nil, booleans, numbers, strings, collections, functions
4. [Literals](literals.md) - Syntax for writing literal values
5. [Symbols and Evaluation](evaluation.md) - Symbol resolution, evaluation rules, quoting

### Language Constructs

6. [Special Forms](special-forms.md) - `def`, `let`, `if`, `do`, `fn`, `quote`, `syntax-quote`
7. [Operators](operators.md) - Arithmetic, comparison, bitwise, logical operators
8. [Functions](functions.md) - Defining, calling, arity, higher-order functions, recursion

### Built-in Functions

9. [Built-in Functions](builtins/index.md) - Native functions implemented in Rust
   - [Type Predicates](builtins/type-predicates.md) - `nil?`, `list?`, `fn?`, etc.
   - [Collections](builtins/collections.md) - `cons`, `first`, `rest`, vector/map/set operations
   - [Binary Operations](builtins/binary.md) - Raw byte buffer operations
   - [Symbols](builtins/symbols.md) - `symbol`, `gensym`
   - [Metadata](builtins/metadata.md) - `meta`, `with-meta`, `vary-meta`
   - [Sorted Collections](builtins/sorted-collections.md) - `sorted-map`, `sorted-set`
   - [Hardware Access](builtins/hardware.md) - MMIO, DMA, IRQ primitives
   - [Time](builtins/time.md) - `now-ms`, `send-after`
   - [Atoms](builtins/atoms.md) - `atom`, `swap!`, `reset!`
   - [I/O](builtins/io.md) - `print`
   - [Processes](builtins/processes.md) - `spawn`, `send`, seL4 operations
   - [Standard Library](builtins/stdlib.md) - Functions implemented in Lonala
   - [Regular Expressions](builtins/regex.md) - `re-pattern`, `re-find`, `re-seq`
   - [Error Handling](builtins/error-handling.md) - `ok?`, `unwrap!`, `map-ok`

### Metaprogramming

10. [Reader Macros](reader-macros.md) - `'`, `` ` ``, `~`, `~@`, `#()`, `#'`, `#_`
11. [Macros](macros.md) - `defmacro`, macro patterns, error handling macros

### Modules and Concurrency

12. [Namespaces](namespaces.md) - `ns`, `require`, qualified references *(Planned)*
13. [Concurrency](concurrency.md) - Processes, message passing, supervision *(Planned)*

### Error Handling and Debugging

14. [Error Handling](error-handling.md) - Result tuples vs conditions, when to use each
15. [Debugging](debugging.md) - Two-Mode Architecture, breakpoints, process debugging

### Appendices

- [Appendix A: Grammar](appendices/grammar.md) - Formal EBNF grammar
- [Appendix B: Bytecode Reference](appendices/bytecode.md) - VM instruction set
- [Appendix C: Differences from Clojure](appendices/clojure-differences.md) - Comparison table
- [Appendix D: Reserved Words](appendices/reserved-words.md) - Special symbols and keywords
- [Appendix E: Error Handling Idioms](appendices/error-idioms.md) - Practical patterns

## References

- [Lona Project Goals](../goals.md) - Vision and design philosophy
- [Implementation Roadmap](../roadmap/index.md) - Development roadmap
- [Clojure Reference](https://clojure.org/reference) - Clojure documentation
- [Erlang Reference Manual](https://www.erlang.org/doc/system/reference_manual.html) - Erlang documentation
- [seL4 Documentation](https://docs.sel4.systems/) - seL4 microkernel
