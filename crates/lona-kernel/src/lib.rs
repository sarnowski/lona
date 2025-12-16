// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Kernel abstractions for the Lona runtime.
//!
//! This crate provides the core kernel components that sit between the
//! language implementation (parser, compiler) and the seL4-specific runtime.
//! Most components are 100% host-testable without seL4 dependencies.
//!
//! Current components:
//! - `vm`: The bytecode virtual machine (register-based, like Lua)
//!
//! Future components will include the scheduler, process management,
//! and garbage collector.

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod vm;
