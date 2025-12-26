// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Kernel abstractions for the Lona runtime.
//!
//! This crate provides the core kernel components that sit between the
//! language implementation (parser, compiler) and the seL4-specific runtime.
//! Most components are 100% host-testable without seL4 dependencies.
//!
//! # Components
//!
//! - [`namespace`]: Namespace system for organizing code into named modules
//! - [`vm`]: The bytecode virtual machine (register-based, like Lua)
//!
//! Future components will include the scheduler, process management,
//! and garbage collector.

#![no_std]
#![expect(
    clippy::float_arithmetic,
    reason = "[approved] VM must execute float operations for language semantics"
)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
pub mod namespace;
pub mod vm;
