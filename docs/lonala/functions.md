# Functions
## 8.1 Defining Functions

Functions are created using the `fn` special form:

```clojure
; Anonymous function (single arity)
(fn [x] (* x x))

; Named function (useful for recursion and debugging)
(fn square [x] (* x x))

; Multi-arity function - different implementations for different arg counts
(fn ([x] x)
    ([x y] (+ x y))
    ([x y & rest] (apply + x y rest)))

; Named multi-arity function
(fn my-fn ([x] x) ([x y] (+ x y)))
```

Multi-arity functions select the implementation based on argument count. Exact arity matches take priority over variadic matches.

To give a function a global name, combine `def` and `fn`:

```clojure
(def square (fn [x] (* x x)))
(square 5)  ; => 25

; Or with a name for recursion
(def factorial
  (fn factorial [n]
    (if (<= n 1)
      1
      (* n (factorial (- n 1))))))

; Multi-arity with recursion (tail-recursive accumulator pattern)
(def fact
  (fn fact
    ([n] (fact n 1))
    ([n acc] (if (<= n 1) acc (fact (- n 1) (* n acc))))))
```

## 8.2 Calling Functions

Function calls use list syntax with the function in the first position:

```clojure
(function-name arg1 arg2 ...)
```

Arguments are evaluated left-to-right before being passed to the function:

```clojure
(+ 1 2)              ; call + with arguments 1 and 2
(square 5)           ; call square with argument 5
(+ (square 2) 1)     ; nested: (+ 4 1) => 5
```

## 8.3 Function Arity

Functions can have one or more arities. Single-arity functions require an exact argument count. Multi-arity functions dispatch to the matching arity body:

```clojure
; Single-arity function
(def greet (fn [name] (print name)))
(greet "Alice")      ; OK
(greet)              ; ERROR: wrong arity
(greet "A" "B")      ; ERROR: wrong arity

; Multi-arity function
(def greet
  (fn ([] "Hello!")
      ([name] (str "Hello, " name "!"))))
(greet)              ; => "Hello!"
(greet "Alice")      ; => "Hello, Alice!"
(greet "A" "B")      ; ERROR: no matching arity
```

Variadic arities (using `& rest`) match when no exact arity exists:

```clojure
(def f
  (fn ([x] :one)
      ([x & rest] :many)))
(f 1)                ; => :one (exact match)
(f 1 2)              ; => :many (variadic match)
(f 1 2 3)            ; => :many (variadic match)
```

## 8.4 Function Bodies

Function bodies can contain multiple expressions. The value of the last expression is returned:

```clojure
(def process
  (fn [x]
    (print "Processing...")
    (print x)
    (* x 2)))  ; this value is returned

(process 5)  ; prints messages, returns 10
```

## 8.5 Higher-Order Functions

Functions can accept functions as arguments and return functions:

```clojure
; Function that takes a function
(def apply-twice
  (fn [f x]
    (f (f x))))

(apply-twice (fn [x] (+ x 1)) 5)  ; => 7

; Function that returns a function
(def make-adder
  (fn [n]
    (fn [x] (+ x n))))

(def add-5 (make-adder 5))
(add-5 10)  ; => 15
```

## 8.6 Recursion

Named functions can call themselves recursively:

```clojure
(def sum-to
  (fn sum-to [n]
    (if (<= n 0)
      0
      (+ n (sum-to (- n 1))))))

(sum-to 5)  ; => 15 (5+4+3+2+1)
```

## 8.7 Closures

Functions capture their lexical environment, creating closures:

```clojure
(defn make-adder [n]
  (fn [x] (+ x n)))  ; captures n from enclosing scope

(def add5 (make-adder 5))
(add5 10)  ; => 15
```

**Closure semantics**:
- **Copy semantics**: Values are captured at closure creation time, not by reference
- **Multi-level capture**: Nested closures can access variables from grandparent+ scopes
- **Identity equality**: Two closures with same code but different captures are not equal

## 8.8 Parameter Destructuring

Function parameters support sequential destructuring patterns, enabling direct extraction of values from collection arguments.

```clojure
; Basic destructuring
((fn [[a b]] (+ a b)) [1 2])        ; => 3

; With rest binding
((fn [[x & xs]] xs) [1 2 3 4])      ; => (2 3 4)

; With :as binding
((fn [[a :as all]] all) [1 2 3])    ; => [1 2 3]

; Nested destructuring
((fn [[[x y] z]] (+ x y z)) [[1 2] 3])  ; => 6

; Mixed with simple params
((fn [[a b] c] (+ a b c)) [1 2] 3)  ; => 6

; Ignored parameters with _
((fn [_ x] x) 1 2)                   ; => 2
((fn [x & _] x) 1 2 3 4)            ; => 1
```

**Missing elements** bind to `nil`:
```clojure
((fn [[a b]] b) [1])        ; => nil
((fn [[a]] a) nil)          ; => nil
```

See [Special Forms - fn](special-forms.md#65-fn) for complete documentation.

## 8.9 Planned Features

The following function features are planned for future implementation:

- **Tail call optimization** *(Planned)*: Efficient recursive loops with `recur`

---

