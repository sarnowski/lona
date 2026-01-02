//! Core type definitions for the Lona VM.
//!
//! This module provides type-safe wrappers for addresses, process identifiers,
//! and other fundamental types. Using newtypes prevents mixing incompatible
//! values (e.g., passing a physical address where a virtual address is expected).

mod address;
mod pid;

pub use address::{Paddr, Vaddr};
pub use pid::Pid;
