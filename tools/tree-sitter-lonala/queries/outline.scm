; SPDX-License-Identifier: GPL-3.0-or-later
; Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>
;
; Tree-sitter outline queries for Lonala
;
; Used by editors for code outline/symbols view to show top-level definitions.

; Top-level definitions for outline view (direct children of source_file only)
; Note: defprotocol/deftype/defrecord/defmulti/defmethod are Clojure Java-interop
; features that are not planned for Lonala. defnative is Lonala-specific.
(source_file
  (list
    . (symbol) @context
    . (symbol) @name
    (#any-of? @context "def" "defn" "defn-" "defmacro" "defonce" "defnative")
  ) @item)
