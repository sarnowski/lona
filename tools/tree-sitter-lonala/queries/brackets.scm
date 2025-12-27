; SPDX-License-Identifier: GPL-3.0-or-later
; Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>
;
; Tree-sitter bracket queries for Lonala
;
; Used by editors for bracket matching and auto-closing.

; Opening brackets
("(" @open)
("[" @open)
("{" @open)
(set "#{" @open)
(anon_fn "#(" @open)

; Closing brackets
(")" @close)
("]" @close)
("}" @close)
