; SPDX-License-Identifier: GPL-3.0-or-later
; Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>
;
; Tree-sitter highlight queries for Lonala
;
; Capture Naming Convention (Tree-sitter standard):
; - @number, @string: Literal values
; - @constant.builtin: Language constants (true, false, nil)
; - @variable: Identifiers/symbols
; - @string.special.symbol: Keywords (special string-like identifiers)
; - @comment: Comments and discarded forms
; - @punctuation.bracket: Grouping delimiters
; - @punctuation.special: Reader macro prefixes
; - @operator: Reader macro operators
; - @keyword: Special forms and control flow
; - @function.call: Function invocations
; - @variable.parameter: Anonymous function arguments

; =============================================================================
; Literals
; =============================================================================

(number) @number
(string) @string
(boolean) @constant.builtin
(nil) @constant.builtin

; =============================================================================
; Identifiers
; =============================================================================

(symbol) @variable
(keyword) @string.special.symbol

; =============================================================================
; Comments
; =============================================================================

(comment) @comment

; =============================================================================
; Punctuation - Brackets
; =============================================================================

["(" ")"] @punctuation.bracket
["[" "]"] @punctuation.bracket
["{" "}"] @punctuation.bracket

; =============================================================================
; Punctuation - Special (reader macro prefixes)
; =============================================================================

(set "#{" @punctuation.special)
(anon_fn "#(" @punctuation.special)
(var_quote "#'" @punctuation.special)
(discard "#_" @punctuation.special)

; =============================================================================
; Reader Macro Operators
; =============================================================================

(quote "'" @operator)
(syntax_quote "`" @operator)
(unquote "~" @operator)
(unquote_splice "~@" @operator)
(metadata "^" @operator)

; =============================================================================
; Discarded Forms (show as comments)
; =============================================================================

(discard) @comment

; =============================================================================
; Special Forms (pattern match on first symbol in list)
; =============================================================================

; Definition forms
(list
  . (symbol) @keyword
  (#any-of? @keyword "def" "defn" "defn-" "defmacro" "defonce" "defprotocol" "deftype" "defrecord" "defmulti" "defmethod"))

; Binding forms
(list
  . (symbol) @keyword
  (#any-of? @keyword "let" "fn" "fn*" "loop" "letfn" "binding" "with-local-vars"))

; Control flow forms
(list
  . (symbol) @keyword
  (#any-of? @keyword "if" "if-not" "if-let" "if-some" "when" "when-not" "when-let" "when-some" "when-first" "cond" "condp" "case"))

; Exception handling
(list
  . (symbol) @keyword
  (#any-of? @keyword "try" "catch" "finally" "throw"))

; Looping and recursion
(list
  . (symbol) @keyword
  (#any-of? @keyword "recur" "while" "doseq" "dotimes" "for"))

; Quoting and evaluation
(list
  . (symbol) @keyword
  (#any-of? @keyword "quote" "do" "eval"))

; Threading macros
(list
  . (symbol) @keyword
  (#any-of? @keyword "->" "->>" "as->" "cond->" "cond->>" "some->" "some->>"))

; Namespace forms
(list
  . (symbol) @keyword
  (#any-of? @keyword "ns" "in-ns" "require" "use" "import" "refer"))

; Assertion and debugging
(list
  . (symbol) @keyword
  (#any-of? @keyword "assert" "comment"))

; Special operators (Lonala-specific, no FFI/Java interop)
(list
  . (symbol) @keyword
  (#any-of? @keyword "set!" "var"))

; =============================================================================
; Function calls (symbols at head of list that aren't special forms)
; Note: This is lower priority than the special form matches above
; =============================================================================

(list
  . (symbol) @function.call)

; =============================================================================
; Anonymous function arguments
; Matches %, %1, %2, etc. at various nesting depths within #(...)
; =============================================================================

; Immediate children - matches %, %1, %2, ..., %&
(anon_fn
  (symbol) @variable.parameter
  (#match? @variable.parameter "^%([0-9]*|&)$"))

; One level deep (inside list, vector, map)
(anon_fn
  (_
    (symbol) @variable.parameter
    (#match? @variable.parameter "^%([0-9]*|&)$")))

; Two levels deep
(anon_fn
  (_
    (_
      (symbol) @variable.parameter
      (#match? @variable.parameter "^%([0-9]*|&)$"))))
