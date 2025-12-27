; SPDX-License-Identifier: GPL-3.0-or-later
; Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>
;
; Tree-sitter indentation queries for Lonala
;
; Used by editors for automatic indentation in Lisp-style code.

; Indent after opening any form
(list "(" @indent)
(vector "[" @indent)
(map "{" @indent)
(set "#{" @indent)
(anon_fn "#(" @indent)

; Dedent before closing brackets
(list ")" @outdent)
(vector "]" @outdent)
(map "}" @outdent)
(set "}" @outdent)
(anon_fn ")" @outdent)
