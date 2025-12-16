// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Memory management for the Lona runtime on seL4.
//!
//! Provides integration between Lona's allocator abstraction and seL4's
//! capability-based memory model. The key components are:
//!
//! - [`Sel4PageProvider`] - Implements [`PageProvider`] using seL4 untyped memory
//! - Untyped memory tracking and retyping into frames
//! - `VSpace` mapping of frames into the address space

mod provider;
mod slots;
mod untyped;

pub use provider::Sel4PageProvider;
