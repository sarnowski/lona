# lona-vm

Lona VM - Lonala bytecode virtual machine for seL4 realms.

This crate provides the runtime for Lonala bytecode:
- Heap management for Lonala values
- UART drivers for aarch64 (PL011) and x86_64 (COM1)
- Reader (lexer/parser) for Lonala source code
- Value representation and printing
- Library loading from embedded tar archives
- REPL for interactive development

## Architecture

The VM runs in isolation within a realm's VSpace, communicating
with the Lona Memory Manager only via IPC. It implements BEAM-style
lightweight processes with per-process GC and message passing.

## License

GPL-3.0-or-later
