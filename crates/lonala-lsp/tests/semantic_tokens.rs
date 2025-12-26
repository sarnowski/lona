// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Integration tests for semantic token classification.

use lonala_lsp::document::Document;
use lonala_lsp::semantic_tokens;

fn make_doc(content: &str) -> Document {
    Document::new(content.to_string(), 1)
}

#[test]
fn special_forms_classified_as_keyword() {
    let doc = make_doc("(def x 42)");
    let tokens = semantic_tokens::compute(&doc);

    // First meaningful token should be "def" as KEYWORD
    let def_token = tokens.iter().find(|t| t.length == 3).unwrap();
    assert_eq!(def_token.token_type, semantic_tokens::types::KEYWORD);
}

#[test]
fn symbols_classified_as_variable() {
    let doc = make_doc("map");
    let tokens = semantic_tokens::compute(&doc);

    // Single symbol "map" should be VARIABLE (not a special form)
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].token_type, semantic_tokens::types::VARIABLE);
    assert_eq!(tokens[0].length, 3);
}

#[test]
fn keywords_classified_as_enum_member() {
    let doc = make_doc("{:name \"Alice\"}");
    let tokens = semantic_tokens::compute(&doc);

    let kw_token = tokens.iter().find(|t| t.length == 5).unwrap(); // :name
    assert_eq!(kw_token.token_type, semantic_tokens::types::ENUM_MEMBER);
}

#[test]
fn numbers_classified_correctly() {
    let doc = make_doc("42 3.14");
    let tokens = semantic_tokens::compute(&doc);

    assert!(
        tokens
            .iter()
            .all(|t| t.token_type == semantic_tokens::types::NUMBER)
    );
}

#[test]
fn delta_encoding_multiline() {
    let doc = make_doc("(def x\n  42)");
    let tokens = semantic_tokens::compute(&doc);

    // Should have multiple tokens across lines
    assert!(tokens.len() >= 3);
    // Check that delta_line is used correctly
    assert!(tokens.iter().any(|t| t.delta_line > 0));
}

// Edge case tests

#[test]
fn empty_document() {
    let doc = make_doc("");
    let tokens = semantic_tokens::compute(&doc);

    assert!(tokens.is_empty());
}

#[test]
fn whitespace_only_document() {
    let doc = make_doc("   \n\t  \n  ");
    let tokens = semantic_tokens::compute(&doc);

    assert!(tokens.is_empty());
}

#[test]
fn unicode_in_string() {
    let doc = make_doc("\"héllo 世界 🎉\"");
    let tokens = semantic_tokens::compute(&doc);

    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].token_type, semantic_tokens::types::STRING);
}

#[test]
fn unicode_in_symbol() {
    // Lonala symbols can contain unicode
    let doc = make_doc("café");
    let tokens = semantic_tokens::compute(&doc);

    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].token_type, semantic_tokens::types::VARIABLE);
}

#[test]
fn malformed_input_partial_string() {
    // Unclosed string - lexer will produce an error token
    let doc = make_doc("\"unclosed");
    let tokens = semantic_tokens::compute(&doc);

    // Should handle gracefully (may produce 0 or partial tokens)
    // The key is no panic
    let _ = tokens;
}

#[test]
fn quote_operators_classified_correctly() {
    let doc = make_doc("'x `y ~z ~@w #'v ^m");
    let tokens = semantic_tokens::compute(&doc);

    // Quote operators should be OPERATOR, symbols should be VARIABLE
    let operators: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == semantic_tokens::types::OPERATOR)
        .collect();
    let variables: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == semantic_tokens::types::VARIABLE)
        .collect();

    // 6 quote operators: ' ` ~ ~@ #' ^
    assert_eq!(operators.len(), 6);
    // 6 symbols: x y z w v m
    assert_eq!(variables.len(), 6);
}
