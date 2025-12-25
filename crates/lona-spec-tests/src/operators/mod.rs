// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Specification Tests: Section 7 - Operators
//!
//! Reference: docs/lonala/operators.md
//!
//! Tests arithmetic, comparison, and logical operators.
//!
//! ## Submodules
//!
//! - [`addition`] - Section 7.1.1: Addition (+)
//! - [`subtraction`] - Section 7.1.2: Subtraction (-)
//! - [`multiplication`] - Section 7.1.3: Multiplication (*)
//! - [`division`] - Section 7.1.4: Division (/)
//! - [`modulo`] - Section 7.1.5: Modulo (mod)
//! - [`comparison`] - Section 7.2: Comparison operators (=, <, >, <=, >=)
//! - [`bitwise`] - Section 7.3: Bitwise operators
//! - [`coercion`] - Section 7.5: Numeric type coercion
//! - [`first_class`] - Section 7.6-7.7: First-class operators

mod addition;
mod bitwise;
mod coercion;
mod comparison;
mod division;
mod first_class;
mod modulo;
mod multiplication;
mod subtraction;
