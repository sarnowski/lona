// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Documentation helpers for special forms and language constructs.
//!
//! This module provides utilities for identifying and documenting special forms,
//! which are compiler primitives that cannot carry `:doc` metadata like regular
//! Vars.

/// Check if a symbol name is a special form.
///
/// Special forms are compiler primitives that don't have Vars and cannot
/// carry `:doc` metadata. They require hardcoded documentation.
///
/// Note: Qualified symbols (e.g., `ns/if`) are NOT special forms.
#[inline]
#[must_use]
pub fn is_special_form(name: &str) -> bool {
    // Qualified symbols are never special forms
    if name.contains('/') {
        return false;
    }
    // Derive from special_form_doc to maintain single source of truth
    special_form_doc(name).is_some()
}

/// Get documentation for a special form.
///
/// Returns `None` for non-special-forms. Use Var metadata for
/// functions, macros, and natives.
#[inline]
#[must_use]
pub fn special_form_doc(name: &str) -> Option<&'static str> {
    match name {
        "def" => Some(
            "Binds a value to a symbol.\n\nSyntax: (def name value)\n       (def name \"docstring\" value)",
        ),
        "let" => Some("Creates local bindings.\n\nSyntax: (let [bindings*] body*)"),
        "if" => Some("Conditional branching.\n\nSyntax: (if test then else?)"),
        "do" => Some("Sequential execution, returns last value.\n\nSyntax: (do exprs*)"),
        "fn" => Some(
            "Creates a function.\n\nSyntax: (fn name? [params*] body*)\n       (fn name? ([params*] body*)+)",
        ),
        "loop" => Some("Loop with recur target.\n\nSyntax: (loop [bindings*] body*)"),
        "recur" => Some("Jump to nearest loop/fn with new values.\n\nSyntax: (recur exprs*)"),
        "quote" => Some("Returns form unevaluated.\n\nSyntax: 'form or (quote form)"),
        "var" => Some("Returns the Var for a symbol.\n\nSyntax: #'symbol or (var symbol)"),
        "try" => Some("Exception handling.\n\nSyntax: (try expr* (catch type binding expr*)*)"),
        "catch" => Some("Catch clause within try.\n\nSyntax: (catch type binding expr*)"),
        "throw" => Some("Throws an exception.\n\nSyntax: (throw expr)"),
        "case" => Some("Value-based dispatch.\n\nSyntax: (case expr clauses* default?)"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_special_forms_recognized() {
        assert!(is_special_form("def"));
        assert!(is_special_form("let"));
        assert!(is_special_form("if"));
        assert!(is_special_form("do"));
        assert!(is_special_form("fn"));
        assert!(is_special_form("loop"));
        assert!(is_special_form("recur"));
        assert!(is_special_form("quote"));
        assert!(is_special_form("var"));
        assert!(is_special_form("try"));
        assert!(is_special_form("catch"));
        assert!(is_special_form("throw"));
        assert!(is_special_form("case"));
    }

    #[test]
    fn test_regular_symbols_not_special_forms() {
        assert!(!is_special_form("map"));
        assert!(!is_special_form("filter"));
        assert!(!is_special_form("reduce"));
        assert!(!is_special_form("x"));
        assert!(!is_special_form("my-function"));
    }

    #[test]
    fn test_qualified_symbols_not_special_forms() {
        assert!(!is_special_form("ns/if"));
        assert!(!is_special_form("core/def"));
        assert!(!is_special_form("my.ns/let"));
    }

    #[test]
    fn test_special_form_doc_returns_documentation() {
        assert!(special_form_doc("if").is_some());
        assert!(special_form_doc("def").is_some());
        assert!(special_form_doc("fn").is_some());

        let if_doc = special_form_doc("if").unwrap();
        assert!(if_doc.contains("Conditional"));
        assert!(if_doc.contains("Syntax"));
    }

    #[test]
    fn test_special_form_doc_returns_none_for_non_special_forms() {
        assert!(special_form_doc("map").is_none());
        assert!(special_form_doc("ns/if").is_none());
    }
}
