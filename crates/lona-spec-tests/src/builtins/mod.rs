// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 9 - Built-in Functions
//!
//! Reference: docs/lonala.md#9-built-in-functions
//!
//! Tests built-in functions (primitives/natives) implemented in Rust.
//! Split into submodules by function category.

mod atoms;
mod binary;
mod collections;
mod io;
mod metadata;
mod regex;
mod sorted_collections;
mod symbols;
mod type_predicates;
