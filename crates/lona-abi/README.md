# lona-abi

Shared ABI definitions between Lona Memory Manager and Lona VM.

This crate defines the contract between the two Lona binaries:
- Type definitions for IDs, addresses, and capabilities
- VSpace layout constants (fixed virtual addresses for all regions)
- IPC message formats for fault handling and requests
- Boot protocol (entry point arguments)

## Design Principles

- **No dependencies**: Pure data types, 100% host-testable
- **Stable layout**: All types use `#[repr(C)]` for FFI safety
- **64-bit only**: Lona targets 64-bit platforms exclusively

## License

GPL-3.0-or-later
