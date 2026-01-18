// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for intrinsic cost function.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{id, intrinsic_cost};

#[test]
fn arithmetic_ops_cost_one() {
    assert_eq!(intrinsic_cost(id::ADD), 1);
    assert_eq!(intrinsic_cost(id::SUB), 1);
    assert_eq!(intrinsic_cost(id::MUL), 1);
    assert_eq!(intrinsic_cost(id::DIV), 1);
    assert_eq!(intrinsic_cost(id::MOD), 1);
}

#[test]
fn comparison_ops_cost_one() {
    assert_eq!(intrinsic_cost(id::EQ), 1);
    assert_eq!(intrinsic_cost(id::LT), 1);
    assert_eq!(intrinsic_cost(id::GT), 1);
    assert_eq!(intrinsic_cost(id::LE), 1);
    assert_eq!(intrinsic_cost(id::GE), 1);
    assert_eq!(intrinsic_cost(id::NOT), 1);
}

#[test]
fn predicate_ops_cost_one() {
    assert_eq!(intrinsic_cost(id::IS_NIL), 1);
    assert_eq!(intrinsic_cost(id::IS_INT), 1);
    assert_eq!(intrinsic_cost(id::IS_STR), 1);
    assert_eq!(intrinsic_cost(id::IS_KEYWORD), 1);
    assert_eq!(intrinsic_cost(id::IS_SYMBOL), 1);
    assert_eq!(intrinsic_cost(id::IS_TUPLE), 1);
    assert_eq!(intrinsic_cost(id::IS_MAP), 1);
    assert_eq!(intrinsic_cost(id::IS_VECTOR), 1);
    assert_eq!(intrinsic_cost(id::IS_NAMESPACE), 1);
    assert_eq!(intrinsic_cost(id::IS_FN), 1);
    assert_eq!(intrinsic_cost(id::IS_VAR), 1);
    assert_eq!(intrinsic_cost(id::IDENTICAL), 1);
}

#[test]
fn simple_collection_ops_cost_two() {
    assert_eq!(intrinsic_cost(id::COUNT), 2);
    assert_eq!(intrinsic_cost(id::FIRST), 2);
    assert_eq!(intrinsic_cost(id::IS_EMPTY), 2);
}

#[test]
fn collection_access_ops_cost_three() {
    assert_eq!(intrinsic_cost(id::NTH), 3);
    assert_eq!(intrinsic_cost(id::GET), 3);
    assert_eq!(intrinsic_cost(id::KEYS), 3);
    assert_eq!(intrinsic_cost(id::VALS), 3);
    assert_eq!(intrinsic_cost(id::REST), 3);
}

#[test]
fn string_ops_cost_three() {
    assert_eq!(intrinsic_cost(id::KEYWORD), 3);
    assert_eq!(intrinsic_cost(id::NAME), 3);
    assert_eq!(intrinsic_cost(id::NAMESPACE), 3);
    assert_eq!(intrinsic_cost(id::STR), 3);
}

#[test]
fn metadata_ops_cost_three() {
    assert_eq!(intrinsic_cost(id::META), 3);
    assert_eq!(intrinsic_cost(id::WITH_META), 3);
}

#[test]
fn namespace_var_ops_cost_ten() {
    assert_eq!(intrinsic_cost(id::CREATE_NS), 10);
    assert_eq!(intrinsic_cost(id::FIND_NS), 10);
    assert_eq!(intrinsic_cost(id::NS_NAME), 10);
    assert_eq!(intrinsic_cost(id::NS_MAP), 10);
    assert_eq!(intrinsic_cost(id::INTERN), 10);
    assert_eq!(intrinsic_cost(id::VAR_GET), 10);
    assert_eq!(intrinsic_cost(id::DEF_ROOT), 10);
    assert_eq!(intrinsic_cost(id::DEF_BINDING), 10);
    assert_eq!(intrinsic_cost(id::DEF_META), 10);
}

#[test]
fn collection_mutation_ops_cost_five() {
    assert_eq!(intrinsic_cost(id::PUT), 5);
}

#[test]
fn unknown_intrinsic_costs_five() {
    // Unknown intrinsic IDs default to cost 5
    assert_eq!(intrinsic_cost(255), 5);
    assert_eq!(intrinsic_cost(200), 5);
}
