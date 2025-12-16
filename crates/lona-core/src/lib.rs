// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Foundational types and traits for the Lona runtime.
//!
//! This crate provides core abstractions that are independent of the seL4
//! platform, enabling thorough testing on the host development machine.
//! All types here are `no_std` compatible and designed for use in a
//! bare-metal environment.
//!
//! # Modules
//!
//! - [`allocator`] - Memory allocation primitives including a bump allocator
//! - [`symbol`] - Symbol interning for efficient identifier handling
//! - [`value`] - Core value types for the Lonala language

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod allocator;
pub mod symbol;
pub mod value;
