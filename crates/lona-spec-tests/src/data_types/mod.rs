// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 3 - Data Types
//!
//! Reference: docs/lonala.md#3-data-types
//!
//! Tests the semantic behavior of Lonala data types as specified.
//! Split into submodules by type category.

mod binary;
mod collections;
mod error_tuples;
mod function;
mod keyword;
mod metadata;
mod primitives;
mod semantics;
mod set;
mod strings;
mod symbols;
