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

All comparison operators return boolean values.

### 7.2.1 Equality: `=`

**Syntax**: `(= x y)`

Returns `true` if x and y are equal.

```clojure
(= 1 1)         ; => true
(= 1 2)         ; => false
(= "a" "a")     ; => true
(= [1 2] [1 2]) ; => true
(= 1 1.0)       ; => true (numeric equality)
```

### 7.2.2 Less Than: `<`

**Syntax**: `(< x y)`

Returns `true` if x is less than y.

```clojure
(< 1 2)         ; => true
(< 2 1)         ; => false
(< 1 1)         ; => false
```

### 7.2.3 Greater Than: `>`

**Syntax**: `(> x y)`

Returns `true` if x is greater than y.

```clojure
(> 2 1)         ; => true
(> 1 2)         ; => false
(> 1 1)         ; => false
```

### 7.2.4 Less Than or Equal: `<=`

**Syntax**: `(<= x y)`

Returns `true` if x is less than or equal to y.

```clojure
(<= 1 2)        ; => true
(<= 1 1)        ; => true
(<= 2 1)        ; => false
```

### 7.2.5 Greater Than or Equal: `>=`

**Syntax**: `(>= x y)`

Returns `true` if x is greater than or equal to y.

```clojure
(>= 2 1)        ; => true
(>= 1 1)        ; => true
(>= 1 2)        ; => false
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

