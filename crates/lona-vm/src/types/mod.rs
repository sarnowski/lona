// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Core type definitions for the Lona VM.
//!
//! This module re-exports address types from `lona-abi` for use within the VM.
//! The canonical definitions live in `lona-abi` to ensure both the VM and memory
//! manager use identical type definitions.

pub use lona_abi::{Paddr, Vaddr};
