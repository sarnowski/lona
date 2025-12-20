# Operators
Operators in Lonala are implemented as functions but have special syntax support for common operations.

## 7.1 Arithmetic Operators

### 7.1.1 Addition: `+`

**Syntax**: `(+ args*)`

Adds numbers together. With no arguments, returns 0.

```clojure
(+)             ; => 0
(+ 1)           ; => 1
(+ 1 2)         ; => 3
(+ 1 2 3 4)     ; => 10
(+ 1.5 2.5)     ; => 4.0
(+ 1 1/2)       ; => 3/2
```

### 7.1.2 Subtraction: `-`

**Syntax**: `(- x)` or `(- x ys*)`

With one argument, returns its negation. With multiple arguments, subtracts subsequent values from the first.

```clojure
(- 5)           ; => -5 (negation)
(- 10 3)        ; => 7
(- 10 3 2)      ; => 5
(- 1.5)         ; => -1.5
```

### 7.1.3 Multiplication: `*`

**Syntax**: `(* args*)`

Multiplies numbers together. With no arguments, returns 1.

```clojure
(*)             ; => 1
(* 5)           ; => 5
(* 2 3)         ; => 6
(* 2 3 4)       ; => 24
(* 1/2 1/3)     ; => 1/6
```

### 7.1.4 Division: `/`

**Syntax**: `(/ x)` or `(/ x ys*)`

With one argument, returns its reciprocal. With multiple arguments, divides the first by the product of the rest.

```clojure
(/ 2)           ; => 1/2 (reciprocal)
(/ 10 2)        ; => 5
(/ 10 2 5)      ; => 1
(/ 1 3)         ; => 1/3 (exact ratio)
(/ 1.0 3)       ; => 0.333... (float)
```

### 7.1.5 Modulo: `mod`

**Syntax**: `(mod x y)`

Returns the remainder of dividing x by y.

```clojure
(mod 10 3)      ; => 1
(mod 10 5)      ; => 0
(mod -10 3)     ; => -1
```

## 7.2 Comparison Operators

All comparison operators return boolean values. Ordering operators (`<`, `>`, `<=`, `>=`) support both numeric types and strings, using lexicographic ordering for strings.

### 7.2.1 Equality: `=`

**Syntax**: `(= x y)` or `(= x y z ...)`

Returns `true` if all arguments are equal. With 0 or 1 arguments, returns `true` (vacuously).

```clojure
(= 1 1)         ; => true
(= 1 2)         ; => false
(= "a" "a")     ; => true
(= [1 2] [1 2]) ; => true
(= 1 1.0)       ; => true (numeric equality)
(= 1 1 1)       ; => true (multi-argument)
(=)             ; => true (vacuously)
(= 42)          ; => true (vacuously)
```

### 7.2.2 Less Than: `<`

**Syntax**: `(< x y)` or `(< x y z ...)`

Returns `true` if arguments are in strictly increasing order. Supports numbers and strings.

```clojure
;; Numeric comparison
(< 1 2)         ; => true
(< 2 1)         ; => false
(< 1 1)         ; => false

;; String comparison (lexicographic)
(< "a" "b")     ; => true
(< "apple" "banana") ; => true
(< "A" "a")     ; => true (UTF-8 byte order)

;; Multi-argument chaining
(< 1 2 3)       ; => true (all increasing)
(< "a" "b" "c") ; => true
(< 1 3 2)       ; => false (3 > 2 breaks chain)
```

### 7.2.3 Greater Than: `>`

**Syntax**: `(> x y)` or `(> x y z ...)`

Returns `true` if arguments are in strictly decreasing order. Supports numbers and strings.

```clojure
;; Numeric comparison
(> 2 1)         ; => true
(> 1 2)         ; => false
(> 1 1)         ; => false

;; String comparison (lexicographic)
(> "b" "a")     ; => true
(> "z" "a")     ; => true

;; Multi-argument chaining
(> 3 2 1)       ; => true (all decreasing)
```

### 7.2.4 Less Than or Equal: `<=`

**Syntax**: `(<= x y)` or `(<= x y z ...)`

Returns `true` if arguments are in non-decreasing order. Supports numbers and strings.

```clojure
(<= 1 2)        ; => true
(<= 1 1)        ; => true
(<= 2 1)        ; => false
(<= "a" "a")    ; => true
(<= "a" "b")    ; => true
```

### 7.2.5 Greater Than or Equal: `>=`

**Syntax**: `(>= x y)` or `(>= x y z ...)`

Returns `true` if arguments are in non-increasing order. Supports numbers and strings.

```clojure
(>= 2 1)        ; => true
(>= 1 1)        ; => true
(>= 1 2)        ; => false
(>= "b" "a")    ; => true
(>= "a" "a")    ; => true
```

### 7.2.6 Type Requirements

Ordering operators require all arguments to be the same comparable type:
- All numeric (Integer, Float, Ratio can be mixed)
- All strings

Mixing numbers and strings produces a type error:

```clojure
(< 1 "a")       ; => ERROR: cannot compare Integer and String
(< "a" 1)       ; => ERROR: cannot compare String and Integer
```

## 7.3 Bitwise Operators *(Planned)*

Bitwise operations work on integers.

### 7.3.1 Bitwise AND: `bit-and`

**Syntax**: `(bit-and x y)`

Returns the bitwise AND of x and y.

```clojure
(bit-and 0xFF 0x0F)     ; => 15 (0x0F)
(bit-and 0b1100 0b1010) ; => 8 (0b1000)
```

### 7.3.2 Bitwise OR: `bit-or`

**Syntax**: `(bit-or x y)`

Returns the bitwise OR of x and y.

```clojure
(bit-or 0b1100 0b0011)  ; => 15 (0b1111)
```

### 7.3.3 Bitwise XOR: `bit-xor`

**Syntax**: `(bit-xor x y)`

Returns the bitwise XOR of x and y.

```clojure
(bit-xor 0b1100 0b1010) ; => 6 (0b0110)
```

### 7.3.4 Bitwise NOT: `bit-not`

**Syntax**: `(bit-not x)`

Returns the bitwise complement of x.

```clojure
(bit-not 0)             ; => -1
(bit-and (bit-not 0xFF) 0xFFFF) ; => 0xFF00
```

### 7.3.5 Shift Left: `bit-shift-left`

**Syntax**: `(bit-shift-left x n)`

Shifts x left by n bits.

```clojure
(bit-shift-left 1 4)    ; => 16
(bit-shift-left 0xFF 8) ; => 0xFF00
```

### 7.3.6 Shift Right: `bit-shift-right`

**Syntax**: `(bit-shift-right x n)`

Shifts x right by n bits (arithmetic shift, preserves sign).

```clojure
(bit-shift-right 16 2)  ; => 4
(bit-shift-right 0xFF00 8) ; => 0xFF
```

## 7.4 Logical Operators

### 7.4.1 Logical Not: `not`

**Syntax**: `(not x)`

Returns `true` if x is falsy, `false` otherwise.

```clojure
(not false)     ; => true
(not nil)       ; => true
(not true)      ; => false
(not 0)         ; => false (0 is truthy)
(not "")        ; => false (empty string is truthy)
```

## 7.5 Numeric Type Coercion

When operations mix numeric types:

| Operation | Result Type |
|-----------|-------------|
| Integer + Integer | Integer |
| Integer + Float | Float |
| Integer + Ratio | Ratio |
| Float + Ratio | Float |
| Integer / Integer (exact) | Integer |
| Integer / Integer (inexact) | Ratio |

```clojure
(+ 1 2)         ; => 3 (Integer)
(+ 1 2.0)       ; => 3.0 (Float)
(+ 1 1/2)       ; => 3/2 (Ratio)
(+ 1.0 1/2)     ; => 1.5 (Float)
(/ 6 2)         ; => 3 (Integer, exact)
(/ 5 2)         ; => 5/2 (Ratio, inexact)
```

---

