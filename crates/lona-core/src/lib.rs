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
//! - [`integer`] - Hybrid arbitrary-precision integers (requires `alloc` feature)
//! - [`list`] - Cons-cell linked lists (requires `alloc` feature)
//! - [`map`] - Immutable maps using HAMT (requires `alloc` feature)
//! - [`ratio`] - Exact rational numbers (requires `alloc` feature)
//! - [`string`] - Immutable reference-counted strings (requires `alloc` feature)
//! - [`symbol`] - Symbol interning for efficient identifier handling
//! - [`value`] - Core value types for the Lonala language
//! - [`vector`] - Immutable vectors using persistent trie (requires `alloc` feature)

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod allocator;
#[cfg(feature = "alloc")]
mod fnv;
#[cfg(feature = "alloc")]
mod hamt;
#[cfg(feature = "alloc")]
pub mod integer;
#[cfg(feature = "alloc")]
pub mod list;
#[cfg(feature = "alloc")]
pub mod map;
#[cfg(feature = "alloc")]
mod pvec;
#[cfg(feature = "alloc")]
pub mod ratio;
#[cfg(feature = "alloc")]
pub mod string;
pub mod symbol;
pub mod value;
#[cfg(feature = "alloc")]
pub mod vector;
