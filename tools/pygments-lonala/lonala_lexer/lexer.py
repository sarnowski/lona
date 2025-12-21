# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

"""Lonala lexer for Pygments."""

from pygments.lexer import RegexLexer, bygroups, words
from pygments.token import (
    Comment,
    Error,
    Keyword,
    Name,
    Number,
    Operator,
    Punctuation,
    String,
    Whitespace,
)


class LonalaLexer(RegexLexer):
    """Pygments lexer for the Lonala programming language.

    Lonala is a Clojure-inspired Lisp with Erlang concurrency features,
    designed for the Lona operating system.
    """

    name = "Lonala"
    aliases = ["lonala", "lona"]
    filenames = ["*.lona", "*.lonala"]
    mimetypes = ["text/x-lonala"]

    # Special forms (from docs/lonala/special-forms.md)
    special_forms = (
        "def",
        "defmacro",
        "defnative",
        "let",
        "if",
        "do",
        "fn",
        "quote",
        "syntax-quote",
        # Planned special forms
        "receive",
        "signal",
        "restart-case",
        "handler-bind",
        "invoke-restart",
        "panic!",
        "assert!",
    )

    # Concurrency forms (planned)
    concurrency_forms = (
        "spawn",
        "send",
        "self",
        "link",
        "unlink",
        "spawn-link",
        "spawn-monitor",
        "exit",
        "trap-exit",
    )

    # Core macros (from lona/core.lona)
    core_macros = (
        "defn",
        "when",
        "when-not",
        "when-let",
        "if-let",
        "if-not",
        "cond",
        "case",
        "and",
        "or",
        "letfn",
        "binding",
        "loop",
        "recur",
        "try",
        "catch",
        "throw",
        "monitor",
    )

    # Built-in functions (from crates/lona-kernel/src/vm/natives.rs and docs)
    builtins = (
        # Arithmetic
        "mod",
        "inc",
        "dec",
        "abs",
        "min",
        "max",
        "quot",
        "rem",
        # Comparison (handled separately as operators: =, <, >, <=, >=)
        "not",
        "not=",
        # Type predicates
        "nil?",
        "true?",
        "false?",
        "some?",
        "boolean?",
        "number?",
        "integer?",
        "float?",
        "ratio?",
        "pos?",
        "neg?",
        "zero?",
        "even?",
        "odd?",
        "string?",
        "keyword?",
        "symbol?",
        "fn?",
        "coll?",
        "sequential?",
        "associative?",
        "counted?",
        "list?",
        "vector?",
        "map?",
        "set?",
        "binary?",
        # Collections
        "first",
        "rest",
        "next",
        "cons",
        "conj",
        "seq",
        "seq?",
        "list",
        "vector",
        "hash-map",
        "hash-set",
        "sorted-map",
        "sorted-set",
        "get",
        "get-in",
        "assoc",
        "assoc-in",
        "dissoc",
        "update",
        "update-in",
        "contains?",
        "keys",
        "vals",
        "count",
        "empty",
        "empty?",
        "into",
        "merge",
        "concat",
        "flatten",
        # Sequence operations
        "filter",
        "remove",
        "map",
        "mapcat",
        "reduce",
        "reduce-kv",
        "take",
        "drop",
        "take-while",
        "drop-while",
        "partition",
        "reverse",
        "sort",
        "sort-by",
        "group-by",
        "frequencies",
        # Higher-order functions
        "apply",
        "partial",
        "comp",
        "identity",
        "constantly",
        "juxt",
        "complement",
        "fnil",
        # String
        "str",
        "name",
        "namespace",
        "keyword",
        "symbol",
        "gensym",
        "subs",
        "split",
        "join",
        "trim",
        "lower-case",
        "upper-case",
        # I/O
        "print",
        "println",
        "pr",
        "prn",
        "read-string",
        "native-print",
        # Metadata
        "meta",
        "with-meta",
        "vary-meta",
        # Atoms
        "atom",
        "deref",
        "reset!",
        "swap!",
        "compare-and-set!",
        # Type
        "type",
        "type-of",
        "class",
    )

    # Symbol pattern: allows many special characters common in Lisp
    symbol_pattern = r"[a-zA-Z_*+!\-?][a-zA-Z0-9_*+!\-?\']*"

    tokens = {
        "root": [
            # Whitespace
            (r"\s+", Whitespace),
            # Comments
            (r";.*$", Comment.Single),
            # Strings
            (r'"', String.Double, "string"),
            # Characters (Clojure-style)
            (r"\\(newline|space|tab|return|.)", String.Char),
            # Numbers
            (r"-?\d+\.\d+", Number.Float),
            (r"-?0x[0-9a-fA-F]+", Number.Hex),
            (r"-?\d+/\d+", Number),  # Ratios
            (r"-?\d+N?", Number.Integer),
            # Keywords (Clojure-style :keyword)
            (r":[a-zA-Z_*+!\-?][a-zA-Z0-9_*+!\-?\']*", Keyword.Constant),
            # Booleans and nil
            (r"\b(true|false|nil)\b", Keyword.Constant),
            # Special forms - match after opening paren
            (
                words(special_forms, prefix=r"(?<=\()", suffix=r"(?=[\s\)])"),
                Keyword,
            ),
            # Core macros - match after opening paren
            (
                words(core_macros, prefix=r"(?<=\()", suffix=r"(?=[\s\)])"),
                Keyword,
            ),
            # Concurrency forms - match after opening paren
            (
                words(concurrency_forms, prefix=r"(?<=\()", suffix=r"(?=[\s\)])"),
                Keyword.Namespace,
            ),
            # Built-in functions - match after opening paren
            (
                words(builtins, prefix=r"(?<=\()", suffix=r"(?=[\s\)])"),
                Name.Builtin,
            ),
            # Reader macros
            (r"'", Operator),  # quote
            (r"`", Operator),  # syntax-quote
            (r"~@", Operator),  # unquote-splicing
            (r"~", Operator),  # unquote
            (r"@", Operator),  # deref
            (r"\^", Operator),  # metadata
            (r"#'", Operator),  # var quote
            (r"#\(", Punctuation),  # anonymous function
            (r"#\{", Punctuation),  # set literal
            (r"#_", Comment.Special),  # discard
            # Operators (including & for rest args)
            (r"[+\-*/=<>&]+", Operator),
            # Symbols (identifiers)
            (r"[a-zA-Z_*+!\-?][a-zA-Z0-9_*+!\-?\']*", Name),
            # Punctuation
            (r"[\(\)\[\]\{\}]", Punctuation),
            # Catch-all for any other characters
            (r".", Error),
        ],
        "string": [
            (r'\\[\\nrt"]', String.Escape),
            (r'[^"\\]+', String.Double),
            (r'"', String.Double, "#pop"),
        ],
    }
