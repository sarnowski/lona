# Reader

The reader converts text into Lonala data structures. Lonala is homoiconic: programs are represented as data structures that the reader produces.

---

## Symbols

Symbols begin with a non-numeric character and can contain alphanumerics, `*`, `+`, `!`, `-`, `_`, `'`, `?`, `<`, `>`, `=`.

```clojure
'foo      ; => foo
'my-var   ; => my-var  @todo
'*ns*     ; => *ns*
'+limit+  ; => +limit+  @todo
'valid?   ; => valid?  @todo
```

Qualified symbols include a namespace:

```clojure
'my.ns/bar      ; => my.ns/bar
'lona.core/count ; => lona.core/count  @todo
```

Symbol parsing:

```clojure
(symbol? 'foo)        ; => true
(symbol? 'my.ns/bar)  ; => true
```

---

## Keywords

Keywords begin with `:` and evaluate to themselves.

```clojure
:foo    ; => :foo
:ok     ; => :ok
:error  ; => :error  @todo
```

Qualified keywords:

```clojure
:my.ns/bar  ; => :my.ns/bar
```

Keywords are self-evaluating:

```clojure
(keyword? :foo)  ; => true
(= :foo :foo)    ; => true
```

---

## Numeric Literals

### Default Numbers

| Literal | Type |
|---------|------|
| `42`, `-17` | Integer (arbitrary precision) |
| `42N` | BigInt (explicit) |
| `3.14`, `6.022e23` | Float (f64) |
| `22/7` | Ratio |

```clojure
42    ; => 42
-17   ; => -17
42N   ; => 42N  @todo
3.14  ; => 3.14  @todo
22/7  ; => 22/7  @todo
```

### Fixed-Width Integers

| Suffix | Type | Range |
|--------|------|-------|
| `u8` | Unsigned 8-bit | 0–255 |
| `u16` | Unsigned 16-bit | 0–65535 |
| `u32` | Unsigned 32-bit | 0–2³²-1 |
| `u64` | Unsigned 64-bit | 0–2⁶⁴-1 |
| `i8` | Signed 8-bit | -128–127 |
| `i16` | Signed 16-bit | -32768–32767 |
| `i32` | Signed 32-bit | -2³¹–2³¹-1 |
| `i64` | Signed 64-bit | -2⁶³–2⁶³-1 |

```clojure
;; @todo
42u8     ; => 42u8
0xFFu32  ; => 255u32
-50i16   ; => -50i16
255u8    ; => 255u8
0u8      ; => 0u8
```

### Floating-Point

| Suffix | Type |
|--------|------|
| (none) | f64 (default) |
| `f32` | f32 |
| `f64` | f64 (explicit) |

```clojure
;; @todo
3.14       ; => 3.14
3.14f32    ; => 3.14f32
3.14f64    ; => 3.14
6.022e23   ; => 6.022e23
1.5e-10    ; => 1.5e-10
```

### Base Prefixes

| Prefix | Base |
|--------|------|
| `0x` | Hexadecimal |
| `0o` | Octal |
| `0b` | Binary |
| `Nr` | Radix N |

```clojure
;; @todo
0xFF    ; => 255
0o10    ; => 8
0b1010  ; => 10
2r1111  ; => 15
16rFF   ; => 255
36rZZ   ; => 1295
```

Invalid radix:

```clojure
;; @todo
(read-string "37rABC")  ; => ERROR :invalid-radix
(read-string "1rABC")   ; => ERROR :invalid-radix
;; Radix must be between 2 and 36
```

### Underscores

Numeric literals support `_` for readability:

```clojure
;; @todo
1_000_000         ; => 1000000
0xFF_FF_FF_FFu32  ; => 4294967295u32
0b1111_0000u8     ; => 240u8
```

### Special Floats

```clojure
;; @todo
##Inf   ; => ##Inf
##-Inf  ; => ##-Inf
##NaN   ; => ##NaN
```

Special float predicates:

```clojure
;; @todo
(= ##Inf ##Inf)    ; => true
(= ##NaN ##NaN)    ; => false
(< ##-Inf 0)       ; => true
(> ##Inf 0)        ; => true
```

---

## Strings

Double-quoted, UTF-8 encoded:

```clojure
"Hello, World!"   ; => "Hello, World!"
"Line 1\nLine 2"  ; => "Line 1\nLine 2"
""                ; => ""
```

Escape sequences: `\n`, `\t`, `\r`, `\\`, `\"`, `\uNNNN` (Unicode).

```clojure
"tab:\there"      ; => "tab:\there"
"quote:\"hi\""    ; => "quote:\"hi\""
"backslash:\\"    ; => "backslash:\\"
"unicode:\u0041"  ; => "unicode:A"  @todo
```

Invalid escape sequences:

```clojure
;; @todo
(read-string "\"\\q\"")  ; => ERROR :invalid-escape
(read-string "\"\\x\"")  ; => ERROR :invalid-escape
```

---

## Characters

Prefixed with `\`:

```clojure
;; @todo
\a        ; => \a
\newline  ; => \newline
\space    ; => \space
\tab      ; => \tab
\return   ; => \return
\u0041    ; => \A
```

Character equality:

```clojure
;; @todo
(= \a \a)          ; => true
(= \newline \u000A) ; => true
(= \A \u0041)      ; => true
```

---

## Collections

| Syntax | Type | Characteristics |
|--------|------|-----------------|
| `(1 2 3)` | List | Linked, O(1) prepend |
| `[1 2 3]` | Tuple | Fixed-size, O(1) access |
| `{1 2 3}` | Vector | Persistent, O(log₃₂n) update |
| `%{:a 1}` | Map | Key-value pairs |
| `#{1 2 3}` | Set | Unique elements |

```clojure
'(1 2 3)           ; => (1 2 3)
[1 2 3]            ; => [1 2 3]
{1 2 3}            ; => {1 2 3}
%{:a 1 :b 2}       ; => %{:a 1 :b 2}
#{1 2 3}           ; => #{1 2 3}  @todo
```

Empty collections:

```clojure
'()   ; => ()  @todo
[]    ; => []
{}    ; => {}
%{}   ; => %{}
#{}   ; => #{}  @todo
```

Type predicates:

```clojure
(list? '(1 2 3))    ; => true  @todo
(tuple? [1 2 3])    ; => true
(vector? {1 2 3})   ; => true
(map? %{:a 1})      ; => true
(set? #{1 2 3})     ; => true  @todo
```

---

## Binary Literals

### Byte Sequences

```clojure
;; @todo
#bytes[0x48 0x65 0x6C]  ; => #bytes[0x48 0x65 0x6C]
#bytes"Hello"           ; => #bytes[0x48 0x65 0x6C 0x6C 0x6F]
#bytes[]                ; => #bytes[]
```

Encoding variants:

```clojure
;; @todo
#bytes/ascii"Hello"   ; => #bytes[0x48 0x65 0x6C 0x6C 0x6F]
#bytes/latin1"café"   ; => #bytes[0x63 0x61 0x66 0xE9]
```

Binary predicates:

```clojure
;; @todo
(binary? #bytes[1 2 3])  ; => true
(binary? #bytes"hi")     ; => true
(binary? "hi")           ; => false
```

### Bit Syntax

For binary pattern matching and construction:

```clojure
#bits[version:4 ihl:4 tos:8 len:16/be]
```

Segment format: `value:size/modifiers`

**Modifiers:**

| Modifier | Description |
|----------|-------------|
| `/be` | Big-endian |
| `/le` | Little-endian |
| `/native` | Platform native |
| `/signed` | Signed integer |
| `/unsigned` | Unsigned (default) |
| `/bytes` | Size in bytes |

```clojure
;; @todo
;; Bit patterns in match expressions
(match #bytes[0x45 0x00 0x00 0x28]
  #bits[version:4 ihl:4 tos:8 len:16/be]
    [version ihl tos len])  ; => [4 5 0 40]
```

---

## Reader Macros

| Syntax | Expansion |
|--------|-----------|
| `'x` | `(quote x)` |
| `#'x` | `(var x)` |
| `` `x `` | Syntax-quote |
| `~x` | Unquote (in syntax-quote) |
| `~@x` | Unquote-splicing |
| `^%{:k v}` | Metadata |
| `^:keyword` | `^%{:keyword true}` |
| `#(...)` | Anonymous function |
| `#_form` | Ignore next form |

### Quote

```clojure
'foo           ; => foo
'(1 2 3)       ; => (1 2 3)
'(+ 1 2)       ; => (+ 1 2)
(quote foo)    ; => foo
(quote (1 2))  ; => (1 2)
```

Quote prevents evaluation:

```clojure
(list? '(+ 1 2))       ; => true  @todo
(= '(+ 1 2) '(+ 1 2))  ; => true
```

### Var Reference

```clojure
;; @todo
(def x 42)
(var? #'x)     ; => true
(var-get #'x)  ; => 42
```

### Anonymous Functions

```clojure
;; @todo
(#(+ % 1) 5)       ; => 6
(#(+ %1 %2) 3 4)   ; => 7
(#(apply + %&) 1 2 3)  ; => 6
```

**Placeholder rules:**

| Placeholder | Meaning |
|-------------|---------|
| `%` | First argument (same as `%1`) |
| `%1`, `%2`, ... `%n` | Nth argument (1-indexed) |
| `%&` | Rest arguments (variadic) |

Multiple placeholders:

```clojure
;; @todo
(#(* %1 %2 %3) 2 3 4)  ; => 24
(#(- %2 %1) 3 10)      ; => 7
```

**Arity inference:**
- Determined by highest numbered placeholder used
- `#(+ %1 %3)` creates a 3-arity function (`%2` is unused but valid)

**Restrictions:**
- `%` and `%1` cannot both appear in the same function (reader error)
- Anonymous functions cannot be nested: `#(#(+ % 1) %)` is a reader error
- `%&` can combine with numbered placeholders: `#(apply + %1 %&)` → `(fn* [a & rest] (apply + a rest))`

Rest arguments with numbered placeholders:

```clojure
;; @todo
(#(apply + %1 %&) 10 1 2 3)  ; => 16
```

Anonymous function errors:

```clojure
;; @todo
;; Nested anonymous functions are a reader error
(read-string "#(#(+ % 1) %)")  ; => ERROR :nested-anon-fn

;; Mixing % and %1 is a reader error
(read-string "#(+ % %1)")  ; => ERROR :mixed-placeholders
```

### Syntax-Quote

Syntax-quote resolves symbols to their namespace and enables template construction:

```clojure
`(foo ~x ~@xs)
```

Unquote and unquote-splicing:

```clojure
;; @todo
(def x 42)
(def xs '(1 2 3))
`~x           ; => 42
`(a ~x b)     ; => (a 42 b)
`(a ~@xs b)   ; => (a 1 2 3 b)
```

### Discard (Ignore Form)

```clojure
;; @todo
[1 #_2 3]       ; => [1 3]
[1 #_(+ 1 1) 3] ; => [1 3]
(+ 1 #_2 3)     ; => 4
```

---

## Comments

```clojure
; Line comment

#_ (ignored form)
```

Line comments are ignored by reader:

```clojure
(+ 1 2)  ; this is a comment  ; => 3
```

---

## Nil and Booleans

```clojure
nil    ; => nil
true   ; => true
false  ; => false
```

Boolean predicates:

```clojure
(nil? nil)      ; => true
(true? true)    ; => true  @todo
(false? false)  ; => true  @todo
(boolean? true) ; => true  @todo
(boolean? false) ; => true  @todo
(boolean? nil)  ; => false  @todo
```

---

## Reader Errors

Unterminated input:

```clojure
;; @todo
(read-string "(")           ; => ERROR :unexpected-eof
(read-string "[1 2")        ; => ERROR :unexpected-eof
(read-string "\"hello")     ; => ERROR :unexpected-eof
(read-string "{1 2")        ; => ERROR :unexpected-eof
```

Unmatched delimiters:

```clojure
;; @todo
(read-string ")")           ; => ERROR :unmatched-delimiter
(read-string "]")           ; => ERROR :unmatched-delimiter
(read-string "}")           ; => ERROR :unmatched-delimiter
```
