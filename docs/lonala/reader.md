# Reader

The reader converts text into Lonala data structures. Lonala is homoiconic: programs are represented as data structures that the reader produces.

---

## Symbols

Symbols begin with a non-numeric character and can contain alphanumerics, `*`, `+`, `!`, `-`, `_`, `'`, `?`, `<`, `>`, `=`.

```clojure
foo
my-var
*ns*
+limit+
valid?
```

Qualified symbols include a namespace:

```clojure
my.ns/bar
lona.core/count
```

---

## Keywords

Keywords begin with `:` and evaluate to themselves.

```clojure
:foo
:ok
:error
```

Qualified keywords:

```clojure
:my.ns/bar
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
42u8
0xFFu32
-50i16
```

### Floating-Point

| Suffix | Type |
|--------|------|
| (none) | f64 (default) |
| `f32` | f32 |
| `f64` | f64 (explicit) |

### Base Prefixes

| Prefix | Base |
|--------|------|
| `0x` | Hexadecimal |
| `0o` | Octal |
| `0b` | Binary |
| `Nr` | Radix N |

```clojure
0xFF          ; 255
0o10          ; 8
0b1010        ; 10
2r1111        ; 15
16rFF         ; 255
```

### Underscores

Numeric literals support `_` for readability:

```clojure
1_000_000
0xFF_FF_FF_FFu32
0b1111_0000u8
```

### Special Floats

```clojure
##Inf         ; Positive infinity
##-Inf        ; Negative infinity
##NaN         ; Not a number
```

---

## Strings

Double-quoted, UTF-8 encoded:

```clojure
"Hello, World!"
"Line 1\nLine 2"
```

Escape sequences: `\n`, `\t`, `\r`, `\\`, `\"`, `\uNNNN` (Unicode).

---

## Characters

Prefixed with `\`:

```clojure
\a
\newline
\space
\tab
\return
\uNNNN        ; Unicode code point
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

---

## Binary Literals

### Byte Sequences

```clojure
#bytes[0x48 0x65 0x6C]    ; Explicit bytes
#bytes"Hello"              ; UTF-8 encoded
#bytes/ascii"Hello"        ; ASCII only
#bytes/latin1"Héllo"       ; Latin-1 encoded
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

### Anonymous Functions

```clojure
#(+ % 1)        ; (fn* [x] (+ x 1))
#(+ %1 %2)      ; (fn* [a b] (+ a b))
#(apply + %&)   ; (fn* [& args] (apply + args))
```

**Placeholder rules:**

| Placeholder | Meaning |
|-------------|---------|
| `%` | First argument (same as `%1`) |
| `%1`, `%2`, ... `%n` | Nth argument (1-indexed) |
| `%&` | Rest arguments (variadic) |

**Arity inference:**
- Determined by highest numbered placeholder used
- `#(+ %1 %3)` creates a 3-arity function (`%2` is unused but valid)

**Restrictions:**
- `%` and `%1` cannot both appear in the same function (reader error)
- Anonymous functions cannot be nested: `#(#(+ % 1) %)` is a reader error
- `%&` can combine with numbered placeholders: `#(apply + %1 %&)` → `(fn* [a & rest] (apply + a rest))`

### Syntax-Quote

Syntax-quote resolves symbols to their namespace and enables template construction:

```clojure
`(foo ~x ~@xs)
```

---

## Comments

```clojure
; Line comment

#_ (ignored form)
```

---

## Nil and Booleans

```clojure
nil
true
false
```
