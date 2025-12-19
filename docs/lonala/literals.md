# Literals
This section describes the syntax for writing literal values in source code.

## 4.1 Numeric Literals

### 4.1.1 Integer Literals

```clojure
42              ; decimal
-17             ; negative decimal
0               ; zero

; Alternate bases
0xFF            ; hexadecimal (255)
0xff            ; hexadecimal (case insensitive)
0b1010          ; binary (10)
0o755           ; octal (493)
```

### 4.1.2 Float Literals

```clojure
3.14            ; decimal point required
-0.5            ; negative
.5              ; ERROR: leading decimal point not allowed
0.5             ; correct form

; Scientific notation
1e10            ; 10000000000.0
1E10            ; case insensitive
1.5e-3          ; 0.0015
-2.5e+4         ; -25000.0

; Special values
##Inf           ; positive infinity
##-Inf          ; negative infinity
##NaN           ; not a number
```

### 4.1.3 Ratio Literals

```clojure
1/3             ; one third
22/7            ; ratio (not evaluated as division)
-1/2            ; negative ratio
4/2             ; normalized to 2 (integer)
```

## 4.2 String Literals

Strings are delimited by double quotes:

```clojure
"hello world"
"line 1\nline 2"
"tab\there"
"quote: \"hi\""
"backslash: \\"
""              ; empty string
```

## 4.3 Boolean Literals

```clojure
true
false
```

## 4.4 Nil Literal

```clojure
nil
```

## 4.5 Collection Literals

### 4.5.1 List Literals

Lists are written with parentheses. When evaluated, lists are treated as function calls unless quoted:

```clojure
'()             ; empty list (quoted)
'(1 2 3)        ; list of 1, 2, 3 (quoted)
(list 1 2 3)    ; using list function (planned)
```

### 4.5.2 Vector Literals

```clojure
[]              ; empty vector
[1 2 3]         ; vector of integers
[1, 2, 3]       ; commas optional
[:a :b :c]      ; vector of keywords
```

### 4.5.3 Map Literals

Maps are written with curly braces containing alternating keys and values:

```clojure
{}              ; empty map
{:a 1 :b 2}     ; two key-value pairs
{:a 1, :b 2}    ; commas optional
{"key" "value"} ; string keys
```

### 4.5.4 Set Literals *(Planned)*

Sets are written with `#{}`:

```clojure
#{}             ; empty set
#{1 2 3}        ; set of integers
#{:a :b :c}     ; set of keywords
#{1, 2, 3}      ; commas optional
```

**Note**: Duplicate elements in a set literal are an error:

```clojure
#{1 2 2 3}      ; ERROR: duplicate key 2
```

## 4.6 Symbol Literals

Symbols are written directly without delimiters:

```clojure
foo
bar-baz
*special*
+
->
empty?
update!
ns/qualified
```

## 4.7 Keyword Literals

Keywords begin with a colon:

```clojure
:foo
:bar-baz
:ns/qualified
```

> **Note**: Keywords are parsed but full keyword semantics are planned for future implementation.

---

