// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

module.exports = grammar({
  name: 'lonala',

  extras: $ => [
    /\s/,
    /,/,  // Commas are whitespace in Lonala
    $.comment,
  ],

  rules: {
    source_file: $ => repeat($._form),

    _form: $ => choice(
      // Reader macros (order matters for precedence)
      $.quote,
      $.syntax_quote,
      $.unquote_splice,  // Must come before unquote (longer prefix match)
      $.unquote,
      $.metadata,
      $.var_quote,
      $.discard,
      $.anon_fn,
      // Collections
      $.list,
      $.vector,
      $.map,
      $.set,
      // Literals
      $.number,
      $.string,
      $.boolean,
      $.nil,
      // Identifiers
      $.keyword,
      $.symbol,
    ),

    // Collections
    list: $ => seq('(', repeat($._form), ')'),
    vector: $ => seq('[', repeat($._form), ']'),
    map: $ => seq('{', repeat($._form), '}'),
    set: $ => seq('#{', repeat($._form), '}'),

    // Reader macros
    quote: $ => seq("'", $._form),
    syntax_quote: $ => seq('`', $._form),
    unquote: $ => seq('~', $._form),
    unquote_splice: $ => seq('~@', $._form),
    metadata: $ => seq('^', $._form, $._form),
    var_quote: $ => seq("#'", $.symbol),
    discard: $ => seq('#_', $._form),
    anon_fn: $ => seq('#(', repeat($._form), ')'),

    // Numbers: integers (decimal, hex, binary, octal), floats (decimal, scientific, special), and ratios
    // Use prec(1) to give numbers higher precedence than symbols so -42 parses as number
    // Note: Lonala does NOT allow leading + for numbers (+42 is symbol + followed by 42)
    number: $ => token(prec(1, choice(
      // Special floats: ##NaN, ##Inf, ##-Inf
      /##NaN/,
      /##-?Inf/,
      // Hex: 0xFF, -0xFF (case insensitive)
      /-?0[xX][0-9a-fA-F]+/,
      // Binary: 0b1010, -0b1010 (case insensitive)
      /-?0[bB][01]+/,
      // Octal: 0o755, -0o755 (case insensitive)
      /-?0[oO][0-7]+/,
      // Ratio: 1/3, 22/7, -1/2
      /-?[0-9]+\/[0-9]+/,
      // Float with exponent only: 1e10, -1e10, 1E+5, 2E-3
      /-?[0-9]+[eE][+-]?[0-9]+/,
      // Float with decimal and optional exponent: 3.14, -0.5, 2.5e-3
      /-?[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?/,
      // Integer: 42, -17, 0
      /-?[0-9]+/,
    ))),

    // String literals with escape sequences
    // Supports: \n, \t, \r, \\, \", \0, \uXXXX
    string: $ => seq(
      '"',
      repeat(choice(
        /[^"\\]/,           // Any char except quote or backslash
        /\\[nrt\\"0]/,      // Simple escape sequences
        /\\u[0-9a-fA-F]{4}/, // Unicode escape \uXXXX
      )),
      '"',
    ),

    // Boolean literals - separate from symbols for correct highlighting
    boolean: $ => choice('true', 'false'),

    // Nil literal - separate from symbols for correct highlighting
    nil: $ => 'nil',

    // Keyword: :foo, :ns/name, :café, :λ, :日本語
    // Matches the Rust lexer: : followed by symbol characters
    // Note: :: (auto-resolved) is NOT supported in Lonala - it's Clojure-specific
    //
    // Unicode support: uses negated char class to accept any non-delimiter character.
    // Tree-sitter's regex doesn't support \p{L}, so we match by exclusion.
    // Excluded: whitespace, comma, semicolon, delimiters, quotes, special chars
    keyword: $ => token(seq(
      ':',
      // First char: anything except delimiters/whitespace/digits
      /[^\s,;:()\[\]{}"'`~@^#0-9]/,
      // Continue chars: anything except delimiters/whitespace (digits allowed)
      repeat(/[^\s,;:()\[\]{}"'`~@^#]/),
    )),

    // Symbol: identifiers and operators
    // Must match is_symbol_start/is_symbol_continue from crates/lonala-parser/src/lexer/mod.rs
    // Use prec(-1) to give lower precedence than keywords and numbers
    //
    // Unicode support: uses negated char class to accept any non-delimiter character.
    // Tree-sitter's regex doesn't support \p{L}, so we match by exclusion.
    // This matches: ASCII letters, Unicode letters (café, λ, 日本語, привет), operators
    //
    // Excluded from symbol chars: whitespace, comma, semicolon, delimiters, quotes, special
    // First char also excludes digits (numbers take precedence) and colon (keywords)
    //
    // Special case: + or - followed by digit parses as + symbol then number (not +42 as symbol)
    symbol: $ => token(prec(-1, choice(
      // + or - as standalone operator or followed by non-digit symbol chars
      seq(
        /[+\-]/,
        optional(seq(
          /[^\s,;:()\[\]{}"'`~@^#0-9]/,
          repeat(/[^\s,;:()\[\]{}"'`~@^#]/)
        ))
      ),
      // Regular symbols: first char excludes digits, continue allows digits
      seq(
        /[^\s,;:()\[\]{}"'`~@^#0-9+\-]/,
        repeat(/[^\s,;:()\[\]{}"'`~@^#]/)
      ),
    ))),

    comment: $ => /;.*/,
  }
});
