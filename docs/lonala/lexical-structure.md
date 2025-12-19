# Lexical Structure
## 2.1 Character Set

Lonala source code is UTF-8 encoded. All Unicode characters are valid in strings and comments. Identifiers (symbols) are restricted to a subset of characters.

## 2.2 Whitespace

The following characters are treated as whitespace and serve to separate tokens:

| Character | Name | Code Point |
|-----------|------|------------|
| ` ` | Space | U+0020 |
| `\t` | Tab | U+0009 |
| `\n` | Newline | U+000A |
| `\r` | Carriage Return | U+000D |
| `,` | Comma | U+002C |

Commas are whitespace in Lonala, allowing their optional use for readability:

```clojure
[1, 2, 3]      ; equivalent to [1 2 3]
{:a 1, :b 2}   ; equivalent to {:a 1 :b 2}
```

## 2.3 Comments

Comments begin with a semicolon (`;`) and extend to the end of the line:

```clojure
; This is a comment
(def x 42)  ; inline comment
```

## 2.4 Token Categories

Lonala recognizes the following token types:

| Category | Examples |
|----------|----------|
| Delimiters | `(` `)` `[` `]` `{` `}` |
| Numbers | `42` `-17` `3.14` `1/3` `0xFF` |
| Strings | `"hello"` `"line\nbreak"` |
| Symbols | `foo` `+` `empty?` `ns/name` |
| Keywords | `:foo` `:ns/name` |
| Booleans | `true` `false` |
| Nil | `nil` |
| Reader Macros | `'` `` ` `` `~` `~@` |

---

