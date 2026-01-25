# lona.core

Core language intrinsics. These are native functions implemented in the VM.

---

## Namespace and Vars

### `*ns*`

Process-local var holding current namespace. Each process has its own `*ns*` value,
inherited from the parent at spawn time.

```clojure
(namespace? *ns*)  ; => true
```

### `create-ns`

Create a namespace.

```clojure
(def ns (create-ns 'test.namespace))
(namespace? ns)              ; => true
(= (ns-name ns) 'test.namespace)  ; => true
```

### `find-ns`

Find namespace by symbol.

```clojure
(namespace? (find-ns 'lona.core))  ; => true
(find-ns 'nonexistent.ns)          ; => nil
```

### `ns-name`

Get namespace's symbol name.

```clojure
(symbol? (ns-name (find-ns 'lona.core)))  ; => true
(= (ns-name (find-ns 'lona.core)) 'lona.core)  ; => true
```

### `ns-map`

Get namespace's var bindings.

```clojure
(map? (ns-map (find-ns 'lona.core)))  ; => true  @todo
```

### `intern`

Intern a var in namespace.

```clojure
(def ns (create-ns 'intern.test))
(def v (intern ns 'x 42))
(var? v)       ; => true
(var-get v)    ; => 42
```

### `refer`

Add var references from another namespace.

```clojure
(refer ns)  ; → nil
```

```clojure
;; @todo
(def test-ns (create-ns 'refer.example))
(intern test-ns 'example-fn (fn* [] :example))
(refer 'refer.example)  ; => nil
```

### `alias`

Create namespace alias.

```clojure
(alias alias-sym ns-sym)  ; → nil
```

```clojure
;; @todo
(alias 'core 'lona.core)  ; => nil
;; Now lona.core/+ can be referred to as core/+
```

### `var`

Get var object by name. The `var` special form looks up a var by symbol name
and returns the var object (not its value).

```clojure
(def x 42)
(var? #'x)      ; => true
(var? (var x))  ; => true
```

Note: `var` is a special form - the symbol argument is not evaluated.

### `var-get`

Get value of var.

```clojure
(def y 100)
(var-get #'y)  ; => 100
```

### `meta`

Get metadata of object.

```clojure
(def ^%{:doc "A value"} z 1)
(map? (meta #'z))        ; => true  @todo
(:doc (meta #'z))        ; => "A value"  @todo
(meta 42)                ; => nil
```

### `with-meta`

Return object with new metadata.

```clojure
;; @todo
(def s (with-meta 'foo %{:tag :special}))
(:tag (meta s))  ; => :special
```

---

## Evaluation

### `eval`

Evaluate form.

```clojure
;; @todo
(eval '(+ 1 2))       ; => 3
(eval '[1 2 3])       ; => [1 2 3]
(def a 10)
(eval 'a)             ; => 10
```

### `read-string`

Parse string to form.

```clojure
;; @todo
(read-string "42")         ; => 42
(read-string "(+ 1 2)")    ; => (+ 1 2)
(list? (read-string "(+ 1 2)"))  ; => true
(read-string ":keyword")   ; => :keyword
(read-string "[1 2 3]")    ; => [1 2 3]
(read-string "(")          ; => ERROR :unexpected-eof
(read-string ")")          ; => ERROR :unmatched-delimiter
```

### `macroexpand`

Fully expand macros in form.

```clojure
(macroexpand form)  ; → expanded form
```

```clojure
;; @todo
;; macroexpand fully expands nested macros
(list? (macroexpand '(when true :ok)))  ; => true
```

### `macroexpand-1`

Expand macros once.

```clojure
(macroexpand-1 form)  ; → expanded form
```

```clojure
;; @todo
;; macroexpand-1 only expands the outermost macro once
(list? (macroexpand-1 '(when true :ok)))  ; => true
```

### `gensym`

Generate unique symbol.

```clojure
;; @todo
(symbol? (gensym))           ; => true
(symbol? (gensym "prefix"))  ; => true
(= (gensym) (gensym))        ; => false
```

### `load-file`

Load and evaluate file.

```clojure
(load-file path)  ; → last value
```

```clojure
;; @todo
;; load-file returns error for non-existent file
(load-file "/nonexistent/path.lona")  ; => ERROR :file-not-found
```

---

## Arithmetic

### `+`

Addition.

```clojure
(+ a b ...)  ; → sum
```

```clojure
(+)          ; => 0   @todo
(+ 1)        ; => 1   @todo
(+ 1 2)      ; => 3
(+ 1 2 3 4)  ; => 10  @todo
```

### `-`

Subtraction.

```clojure
(- a)        ; → negation
(- a b ...)  ; → difference
```

```clojure
(- 5)        ; => -5  @todo
(- 10 3)     ; => 7
(- 10 3 2)   ; => 5   @todo
```

### `*`

Multiplication.

```clojure
(* a b ...)  ; → product
```

```clojure
(*)          ; => 1   @todo
(* 3)        ; => 3   @todo
(* 2 3)      ; => 6
(* 2 3 4)    ; => 24  @todo
```

### `/`

Division.

```clojure
(/ a b ...)  ; → quotient
```

```clojure
(/ 10 2)     ; => 5
(/ 20 2 5)   ; => 2   @todo
```

Division by zero:

```clojure
(/ 1 0)      ; => ERROR :division-by-zero
```

### `mod`

Modulus (sign follows divisor).

```clojure
(mod a b)  ; → remainder
```

```clojure
(mod 10 3)   ; => 1
(mod -10 3)  ; => 2
(mod 10 -3)  ; => -2
(mod 10 0)   ; => ERROR :division-by-zero
```

### `quot`

Integer quotient (truncates toward zero).

```clojure
(quot a b)  ; → integer
```

```clojure
;; @todo
(quot 10 3)  ; => 3
(quot -10 3) ; => -3
(quot 10 0)  ; => ERROR :division-by-zero
```

### `rem`

Remainder (sign follows dividend).

```clojure
(rem a b)  ; → remainder
```

```clojure
;; @todo
(rem 10 3)   ; => 1
(rem -10 3)  ; => -1
(rem 10 0)   ; => ERROR :division-by-zero
```

### `checked-add`

Addition with overflow check.

```clojure
;; @todo
(checked-add 1u8 2u8)      ; => [:ok 3u8]
(checked-add 255u8 1u8)    ; => [:error :overflow]
(checked-add 100i8 100i8)  ; => [:error :overflow]
```

### `saturating-add`

Addition clamped to type bounds.

```clojure
;; @todo
(saturating-add 250u8 10u8)   ; => 255u8
(saturating-add 120i8 20i8)   ; => 127i8
(saturating-add -120i8 -20i8) ; => -128i8
```

---

## Ratio

### `numerator`

Get numerator of ratio.

```clojure
;; @todo
(numerator 22/7)   ; => 22
(numerator 1/3)    ; => 1
(numerator 6/4)    ; => 3   ; auto-reduced to 3/2
```

### `denominator`

Get denominator of ratio.

```clojure
;; @todo
(denominator 22/7)  ; => 7
(denominator 1/3)   ; => 3
(denominator 6/4)   ; => 2   ; auto-reduced to 3/2
```

---

## Comparison

### `=`

Equality (value-based).

```clojure
(= 1 1)           ; => true
(= 1 2)           ; => false
(= 1 1 1)         ; => true
(= 1 1 2)         ; => false  @todo
(= :a :a)         ; => true
(= [1 2] [1 2])   ; => true
(= [1 2] [1 3])   ; => false
(= %{:a 1} %{:a 1})  ; => true
```

### `<`

Less than.

```clojure
(< 1 2)       ; => true
(< 2 1)       ; => false
(< 1 1)       ; => false
(< 1 2 3)     ; => true
(< 1 3 2)     ; => false  @todo
```

### `>`

Greater than.

```clojure
(> 2 1)       ; => true
(> 1 2)       ; => false
(> 1 1)       ; => false
(> 3 2 1)     ; => true
(> 3 1 2)     ; => false  @todo
```

### `<=`

Less than or equal.

```clojure
(<= 1 2)      ; => true
(<= 1 1)      ; => true
(<= 2 1)      ; => false
(<= 1 2 2 3)  ; => true
```

### `>=`

Greater than or equal.

```clojure
(>= 2 1)      ; => true
(>= 1 1)      ; => true
(>= 1 2)      ; => false
(>= 3 2 2 1)  ; => true
```

### `not=`

Not equal.

```clojure
;; @todo
(not= 1 2)       ; => true
(not= 1 1)       ; => false
(not= 1 2 3)     ; => true
(not= 1 1 1)     ; => false
```

### `identical?`

Reference identity.

```clojure
(def x [1 2 3])
(identical? x x)          ; => true
(identical? [1 2 3] [1 2 3])  ; => false
(identical? :a :a)        ; => true
```

---

## Boolean

### `not`

Logical negation.

```clojure
(not true)   ; => false
(not false)  ; => true
(not nil)    ; => true
(not 0)      ; => false
(not "")     ; => false
(not '())    ; => false  @todo
```

---

## Collections

### `prepend`

Add element to front of list.

```clojure
;; @todo
(prepend '(2 3) 1)    ; => (1 2 3)
(prepend '() 1)       ; => (1)
(prepend nil 1)       ; => (1)
```

### `append`

Add element to end of vector or tuple.

```clojure
;; @todo
(append {1 2} 3)      ; => {1 2 3}
(append {} 1)         ; => {1}
(append [1 2] 3)      ; => [1 2 3]
(append [] 1)         ; => [1]
```

### `put`

Associate key-value in map.

```clojure
(put %{:a 1} :b 2)        ; => %{:a 1 :b 2}  @todo
(put %{:a 1} :a 99)       ; => %{:a 99}  @todo
(put %{} :x 1)            ; => %{:x 1}
```

### `set-add`

Add element to set.

```clojure
;; @todo
(set-add #{1 2} 3)    ; => #{1 2 3}
(set-add #{1 2} 2)    ; => #{1 2}
(set-add #{} 1)       ; => #{1}
```

### `dissoc`

Remove key from map.

```clojure
;; @todo
(dissoc %{:a 1 :b 2} :a)  ; => %{:b 2}
(dissoc %{:a 1} :b)       ; => %{:a 1}
(dissoc %{} :a)           ; => %{}
```

### `set-remove`

Remove element from set.

```clojure
;; @todo
(set-remove #{1 2 3} 2)   ; => #{1 3}
(set-remove #{1 2} 3)     ; => #{1 2}
(set-remove #{} 1)        ; => #{}
```

### `count`

Number of elements.

```clojure
(count '(1 2 3))      ; => 3
(count [1 2 3])       ; => 3
(count {1 2 3})       ; => 3
(count %{:a 1 :b 2})  ; => 2
(count #{1 2 3})      ; => 3  @todo
(count "hello")       ; => 5
(count '())           ; => 0
(count nil)           ; => 0
```

### `nth`

Get element by index.

```clojure
(nth [1 2 3] 0)           ; => 1
(nth [1 2 3] 2)           ; => 3
(nth {10 20 30} 1)        ; => 20  @todo
(nth [1 2 3] 10 :missing) ; => :missing
(nth '(a b c) 1)          ; => b  @todo
(nth [1 2 3] -1)          ; => nil  @todo
(nth [1 2 3] 100)         ; => nil  @todo
```

### `get`

Get value by key.

```clojure
(get %{:a 1 :b 2} :a)         ; => 1
(get %{:a 1} :b)              ; => nil
(get %{:a 1} :b :default)     ; => :default
(get %{:a nil} :a :default)   ; => nil
```

### `keys`

Get map keys.

```clojure
(list? (keys %{:a 1 :b 2}))   ; => true  @todo
(count (keys %{:a 1 :b 2}))   ; => 2
(keys %{})                    ; => ()  @todo
```

### `vals`

Get map values.

```clojure
(list? (vals %{:a 1 :b 2}))   ; => true  @todo
(count (vals %{:a 1 :b 2}))   ; => 2
(vals %{})                    ; => ()  @todo
```

### `contains?`

Check if map contains key.

```clojure
;; @todo
(contains? %{:a 1 :b 2} :a)   ; => true
(contains? %{:a 1 :b 2} :c)   ; => false
(contains? %{:a nil} :a)      ; => true
(contains? %{} :a)            ; => false
```

Note: Unlike Clojure's `contains?`, this only works on maps (not vectors/sets with indices).

```clojure
;; @todo
;; contains? only works on maps - error on other types
(contains? [1 2 3] 0)    ; => ERROR :type-error
(contains? #{1 2 3} 1)   ; => ERROR :type-error
(contains? "abc" 0)      ; => ERROR :type-error
```

---

## Sequences

These intrinsics are **polymorphic** — they work on any collection type (list, tuple,
vector, map, set). This enables generic functions like `map` and `filter` to be derived
once and work on all collections.

### `first`

First element of any collection.

```clojure
(first '(1 2 3))      ; => 1
(first [1 2 3])       ; => 1
(first {1 2 3})       ; => 1
(first nil)           ; => nil
(first '())           ; => nil
(first [])            ; => nil
```

### `rest`

Remaining elements after first. **Always returns a list**, regardless of input type.

```clojure
(rest '(1 2 3))  ; => (2 3)
(rest [1 2 3])   ; => (2 3)
(rest {1 2 3})   ; => (2 3)
(rest '(1))      ; => ()  @todo
(rest '())       ; => ()  @todo
(rest nil)       ; => ()  @todo
(list? (rest [1 2 3]))  ; => true  @todo
```

### `empty?`

True if collection has no elements.

```clojure
(empty? '())       ; => true
(empty? nil)       ; => true
(empty? [])        ; => true
(empty? {})        ; => true
(empty? %{})       ; => true
(empty? #{})       ; => true  @todo
(empty? '(1))      ; => false
(empty? [1])       ; => false
(empty? "")        ; => true  @todo
(empty? "a")       ; => false  @todo
```

---

## Type Predicates

```clojure
(nil? nil)        ; => true
(nil? false)      ; => false
(boolean? true)   ; => true  @todo
(boolean? false)  ; => true  @todo
(boolean? nil)    ; => false  @todo
(true? true)      ; => true  @todo
(true? false)     ; => false  @todo
(false? false)    ; => true  @todo
(false? true)     ; => false  @todo
(number? 42)      ; => true  @todo
(number? 3.14)    ; => true  @todo
(number? 22/7)    ; => true  @todo
(integer? 42)     ; => true
(integer? 3.14)   ; => false  @todo
(float? 3.14)     ; => true  @todo
(float? 42)       ; => false  @todo
(ratio? 22/7)     ; => true  @todo
(ratio? 1)        ; => false  @todo
(string? "hi")    ; => true
(string? :hi)     ; => false
(char? \a)        ; => true  @todo
(char? "a")       ; => false  @todo
(symbol? 'foo)    ; => true
(symbol? :foo)    ; => false
(keyword? :foo)   ; => true
(keyword? 'foo)   ; => false
(fn? +)           ; => true
(fn? 42)          ; => false
(list? '(1 2))    ; => true  @todo
(list? [1 2])     ; => false  @todo
(tuple? [1 2])    ; => true
(tuple? {1 2})    ; => false
(vector? {1 2})   ; => true
(vector? [1 2])   ; => false
(map? %{:a 1})    ; => true
(map? #{1 2})     ; => false  @todo
(set? #{1 2})     ; => true  @todo
(set? %{:a 1})    ; => false  @todo
```

### `type`

Get type keyword.

```clojure
;; @todo
(type nil)        ; => :nil
(type true)       ; => :boolean
(type 42)         ; => :integer
(type 3.14)       ; => :float
(type 22/7)       ; => :ratio
(type "hi")       ; => :string
(type \a)         ; => :char
(type 'foo)       ; => :symbol
(type :foo)       ; => :keyword
(type '(1 2))     ; => :list
(type [1 2])      ; => :tuple
(type {1 2})      ; => :vector
(type %{:a 1})    ; => :map
(type #{1 2})     ; => :set
```

---

## Type Coercion

Fixed-width integer conversion:

```clojure
;; @todo
(u8 42)          ; => 42u8
(u16 1000)       ; => 1000u16
(u32 100000)     ; => 100000u32
(u64 42)         ; => 42u64
(i8 -50)         ; => -50i8
(i16 -1000)      ; => -1000i16
(i32 -100000)    ; => -100000i32
(i64 -42)        ; => -42i64
```

Overflow wrapping:

```clojure
;; @todo
(u8 256)         ; => 0u8
(u8 -1)          ; => 255u8
(i8 128)         ; => -128i8
(i8 -129)        ; => 127i8
```

Character conversion:

```clojure
;; @todo
(char 65)        ; => \A
(char 97)        ; => \a
(char 10)        ; => \newline
```

---

## Strings

### `str`

Concatenate to string.

```clojure
(str)              ; => ""
(str "a")          ; => "a"
(str "a" "b")      ; => "ab"
(str "a" "b" "c")  ; => "abc"
(str 1 2 3)        ; => "123"
(str :a)           ; => ":a"
(str nil)          ; => ""  @todo
(str "hi " nil " there")  ; => "hi  there"  @todo
```

### `subs`

Substring.

```clojure
;; @todo
(subs "hello" 0)      ; => "hello"
(subs "hello" 1)      ; => "ello"
(subs "hello" 2)      ; => "llo"
(subs "hello" 0 2)    ; => "he"
(subs "hello" 1 4)    ; => "ell"
(subs "hello" 5)      ; => ""
```

### `symbol`

Create symbol.

```clojure
;; @todo
(symbol "foo")           ; => foo
(symbol "my.ns" "bar")   ; => my.ns/bar
(symbol? (symbol "x"))   ; => true
```

### `keyword`

Create keyword.

```clojure
(keyword "foo")          ; => :foo
(keyword "my.ns" "bar")  ; => :my.ns/bar  @todo
(keyword? (keyword "x")) ; => true
```

### `name`

Get name part of symbol/keyword.

```clojure
(name :foo)           ; => "foo"
(name :my.ns/bar)     ; => "bar"
(name 'foo)           ; => "foo"
(name 'my.ns/bar)     ; => "bar"
```

### `namespace`

Get namespace part of symbol/keyword.

```clojure
(namespace :foo)         ; => nil
(namespace :my.ns/bar)   ; => "my.ns"
(namespace 'foo)         ; => nil
(namespace 'my.ns/bar)   ; => "my.ns"
```

---

## Error Handling

Lonala follows the "let it crash" philosophy from Erlang/OTP:

**Use tuple returns** for expected, recoverable errors:
```clojure
[:ok result]      ; Success
[:error reason]   ; Expected failure (invalid input, not found, etc.)
```

**Use `exit`** for unrecoverable errors (see `lona.process/exit`):
```clojure
(exit :normal)                        ; Clean exit (doesn't cascade)
(exit :shutdown)                      ; Clean shutdown
(exit [:error :invariant-violated])   ; Error with reason
```

Crashed processes are restarted by supervisors. Don't catch crashes — let them propagate.

### `make-ref`

Create unique reference (for request/response correlation).

```clojure
;; @todo
(ref? (make-ref))         ; => true
(= (make-ref) (make-ref)) ; => false
```

---

## Function Application

### `apply`

Apply function to args.

```clojure
;; @todo
(apply + '(1 2 3))        ; => 6
(apply + 1 2 '(3 4))      ; => 10
(apply str '("a" "b" "c")) ; => "abc"
(apply * [2 3 4])         ; => 24
(apply 42 '(1 2))         ; => ERROR :badarg
```

---

## Bitwise Operations

### Basic

```clojure
;; @todo
(bit-and 0b1100 0b1010)    ; => 8       ; 0b1000
(bit-and 0xFF 0x0F)        ; => 15
(bit-or 0b1100 0b1010)     ; => 14      ; 0b1110
(bit-or 0 1 2 4)           ; => 7
(bit-xor 0b1100 0b1010)    ; => 6       ; 0b0110
(bit-not 0u8)              ; => 255u8
```

### Shifts

```clojure
;; @todo
(bit-shl 1 4)              ; => 16
(bit-shl 0b0001 3)         ; => 8
(bit-shr 16 2)             ; => 4
(bit-shr 0b1000 3)         ; => 1
(bit-sar -8i32 2)          ; => -2i32
```

### Rotations

```clojure
;; @todo
(bit-rol 0b10000001u8 1 8)  ; => 3u8     ; 0b00000011
(bit-ror 0b10000001u8 1 8)  ; => 192u8   ; 0b11000000
```

### Single-Bit

```clojure
;; @todo
(bit-test 0b1010 1)        ; => true
(bit-test 0b1010 2)        ; => false
(bit-set 0b1000 0)         ; => 9       ; 0b1001
(bit-clear 0b1111 1)       ; => 13      ; 0b1101
(bit-flip 0b1010 1)        ; => 8       ; 0b1000
(bit-flip 0b1010 0)        ; => 11      ; 0b1011
```

### Bit Fields

```clojure
;; @todo
(bit-field 0xABCD 4 8)           ; => 0xBC
(bit-field-set 0 4 8 0xFF)       ; => 0xFF0
```

### Bit Counting

```clojure
;; @todo
(bit-count 0b1010)         ; => 2
(bit-count 0b11111111)     ; => 8
(bit-count 0)              ; => 0
(leading-zeros 1u8)        ; => 7
(trailing-zeros 8u8)       ; => 3
```

### Byte Order

```clojure
;; @todo
(byte-reverse16 0x1234u16)     ; => 0x3412u16
(byte-reverse32 0x12345678u32) ; => 0x78563412u32
```

### Masks

```clojure
;; @todo
(mask-bits 4)              ; => 15      ; 0b1111
(mask-bits 8)              ; => 255
(mask-range 4 8)           ; => 240     ; 0b11110000
```

---

## Binary

### `binary`

Create binary from byte sequence.

```clojure
;; @todo
(def b (binary '(72 101 108 108 111)))
(binary? b)            ; => true
```

### `binary-size`

Size in bytes.

```clojure
;; @todo
(binary-size #bytes"Hello")    ; => 5
(binary-size #bytes[])         ; => 0
(binary-size #bytes[1 2 3])    ; => 3
```

### `binary-ref`

Get byte at offset.

```clojure
;; @todo
(binary-ref #bytes[10 20 30] 0)  ; => 10
(binary-ref #bytes[10 20 30] 1)  ; => 20
(binary-ref #bytes[10 20 30] 2)  ; => 30
(binary-ref #bytes[10 20 30] 3)  ; => ERROR :out-of-bounds
(binary-ref #bytes[10 20 30] -1) ; => ERROR :out-of-bounds
```

### `binary-slice`

Extract slice.

```clojure
;; @todo
(binary-slice #bytes[1 2 3 4 5] 1 3)  ; => #bytes[2 3 4]
(binary-slice #bytes[1 2 3 4 5] 0 2)  ; => #bytes[1 2]
(binary-slice #bytes[1 2 3] 0 0)      ; => #bytes[]
(binary-slice #bytes[1 2 3] 0 10)     ; => ERROR :out-of-bounds
```

### `binary-concat`

Concatenate binaries.

```clojure
;; @todo
(binary-concat #bytes[1 2] #bytes[3 4])  ; => #bytes[1 2 3 4]
(binary-concat #bytes[] #bytes[1])       ; => #bytes[1]
```

### `binary->string`

Decode to string.

```clojure
;; @todo
(binary->string #bytes"Hello")           ; => "Hello"
(binary->string #bytes[72 105])          ; => "Hi"
(binary->string #bytes[72 105] :utf-8)   ; => "Hi"
(binary->string #bytes[0xFF 0xFE] :utf-8) ; => ERROR :invalid-utf8
```

Encodings: `:utf-8` (default), `:ascii`, `:latin1`

### `string->binary`

Encode string.

```clojure
;; @todo
(string->binary "Hi")          ; => #bytes[72 105]
(string->binary "Hi" :utf-8)   ; => #bytes[72 105]
(binary? (string->binary "x")) ; => true
```

Encodings: `:utf-8` (default), `:ascii`, `:latin1`

---

## Bytebuf

### `bytebuf-alloc`

Allocate zeroed buffer.

```clojure
;; @todo
(def buf (bytebuf-alloc 16))
(bytebuf? buf)             ; => true
(bytebuf-size buf)         ; => 16
(bytebuf-read8 buf 0)      ; => 0
```

### `bytebuf-alloc-unsafe`

Allocate uninitialized buffer.

```clojure
;; @todo
(def buf (bytebuf-alloc-unsafe 16))
(bytebuf? buf)             ; => true
(bytebuf-size buf)         ; => 16
```

### `bytebuf-size`

Buffer size.

```clojure
;; @todo
(bytebuf-size (bytebuf-alloc 64))  ; => 64
(bytebuf-size (bytebuf-alloc 0))   ; => 0
```

### `bytebuf->binary`

Convert to immutable binary.

```clojure
;; @todo
(def buf (bytebuf-alloc 4))
(bytebuf-write8! buf 0 1)
(bytebuf-write8! buf 1 2)
(binary? (bytebuf->binary buf))       ; => true
(binary-size (bytebuf->binary buf))   ; => 4
(bytebuf->binary buf 0 2)             ; => #bytes[1 2]
```

### Read Operations

Native endianness:

```clojure
;; @todo
(def buf (bytebuf-alloc 8))
(bytebuf-write8! buf 0 42)
(bytebuf-read8 buf 0)      ; => 42
(bytebuf-read8 buf 1)      ; => 0
```

Little-endian:

```clojure
;; @todo
(def buf (bytebuf-alloc 8))
(bytebuf-write16-le! buf 0 0x1234u16)
(bytebuf-read16-le buf 0)  ; => 0x1234u16
(bytebuf-read8 buf 0)      ; => 0x34
(bytebuf-read8 buf 1)      ; => 0x12
```

Big-endian:

```clojure
;; @todo
(def buf (bytebuf-alloc 8))
(bytebuf-write16-be! buf 0 0x1234u16)
(bytebuf-read16-be buf 0)  ; => 0x1234u16
(bytebuf-read8 buf 0)      ; => 0x12
(bytebuf-read8 buf 1)      ; => 0x34
```

Signed:

```clojure
;; @todo
(def buf (bytebuf-alloc 8))
(bytebuf-write8! buf 0 255)
(bytebuf-read-i8 buf 0)    ; => -1i8
```

### Write Operations

Native endianness:

```clojure
;; @todo
(def buf (bytebuf-alloc 8))
(bytebuf-write8! buf 0 42)     ; => :ok
(bytebuf-write16! buf 2 1000)  ; => :ok
(bytebuf-write32! buf 4 99999) ; => :ok
```

Little-endian:

```clojure
;; @todo
(def buf (bytebuf-alloc 8))
(bytebuf-write16-le! buf 0 0xABCDu16)  ; => :ok
(bytebuf-write32-le! buf 2 0x12345678u32)  ; => :ok
```

Big-endian:

```clojure
;; @todo
(def buf (bytebuf-alloc 8))
(bytebuf-write16-be! buf 0 0xABCDu16)  ; => :ok
(bytebuf-write32-be! buf 2 0x12345678u32)  ; => :ok
```

### Bulk Operations

```clojure
;; @todo
(def src (bytebuf-alloc 4))
(def dst (bytebuf-alloc 4))
(bytebuf-write8! src 0 1)
(bytebuf-write8! src 1 2)
(bytebuf-copy! dst 0 src 0 2)  ; => :ok
(bytebuf-read8 dst 0)          ; => 1
(bytebuf-read8 dst 1)          ; => 2

(bytebuf-fill! dst 0 4 0xFF)   ; => :ok
(bytebuf-read8 dst 0)          ; => 255
```

---

## Endianness

```clojure
;; @todo
(keyword? (native-endian))  ; => true
;; Result is :little or :big depending on platform
```

Endian conversion:

```clojure
;; @todo
;; On little-endian system:
(native->be16 0x1234u16)   ; => 0x3412u16
(be->native16 0x3412u16)   ; => 0x1234u16
(native->le16 0x1234u16)   ; => 0x1234u16
(le->native16 0x1234u16)   ; => 0x1234u16
```

---

## Addresses

### Physical Address

```clojure
;; @todo
(def p (paddr 0x1000u64))
(paddr? p)                     ; => true
(paddr->u64 p)                 ; => 0x1000u64
(paddr->u64 (paddr+ p 0x100u64))  ; => 0x1100u64
(paddr- (paddr 0x2000u64) (paddr 0x1000u64))  ; => 0x1000u64
(paddr= p p)                   ; => true
(paddr< p (paddr 0x2000u64))   ; => true
```

Alignment:

```clojure
;; @todo
(paddr->u64 (paddr-align (paddr 0x1001u64) 0x1000u64))  ; => 0x2000u64
(paddr->u64 (paddr-align-down (paddr 0x1FFFu64) 0x1000u64))  ; => 0x1000u64
(paddr-aligned? (paddr 0x1000u64) 0x1000u64)  ; => true
(paddr-aligned? (paddr 0x1001u64) 0x1000u64)  ; => false
```

### Virtual Address

```clojure
;; @todo
(def v (vaddr 0x4000u64))
(vaddr? v)                     ; => true
(vaddr->u64 v)                 ; => 0x4000u64
(vaddr->u64 (vaddr+ v 0x100u64))  ; => 0x4100u64
(vaddr- (vaddr 0x5000u64) (vaddr 0x4000u64))  ; => 0x1000u64
(vaddr= v v)                   ; => true
(vaddr< v (vaddr 0x5000u64))   ; => true
```

Alignment:

```clojure
;; @todo
(vaddr->u64 (vaddr-align (vaddr 0x1001u64) 0x1000u64))  ; => 0x2000u64
(vaddr->u64 (vaddr-align-down (vaddr 0x1FFFu64) 0x1000u64))  ; => 0x1000u64
(vaddr-aligned? (vaddr 0x1000u64) 0x1000u64)  ; => true
(vaddr-aligned? (vaddr 0x1001u64) 0x1000u64)  ; => false
```

---

## PID

```clojure
;; @todo
(def my-pid (self))
(pid? my-pid)              ; => true
(realm-id? (pid-realm my-pid))  ; => true
(integer? (pid-local my-pid))   ; => true
(pid= my-pid my-pid)       ; => true
```

---

## Capabilities

### Type Predicates

```clojure
;; Capability predicates return boolean
;; (Low-level kernel operations typically create these)
(cap? x)  ; → boolean (generic capability check)
(tcb-cap? x)
(endpoint-cap? x)
(notification-cap? x)
(cnode-cap? x)
(untyped-cap? x)
(frame-cap? x)
(vspace-cap? x)
(sched-context-cap? x)
(irq-handler-cap? x)
(port-cap? x)
```

### Inspection

```clojure
;; @todo
;; cap-type returns type keyword
;; (cap-type tcb)  ; => :tcb
;; (cap-type frame)  ; => :frame

;; cap-rights returns set of rights
;; (set? (cap-rights cap))  ; => true

;; cap-has-right? checks specific right
;; (cap-has-right? cap :read)  ; => true or false
```

---

## Message Info

```clojure
;; @todo
(def mi (msg-info 100 4 2))
(msg-info? mi)         ; => true
(msg-info-label mi)    ; => 100
(msg-info-length mi)   ; => 4
(msg-info-caps mi)     ; => 2
```

---

## Appendix: Expected Derived Functions

The following are **not intrinsics** and should be implemented in Lonala:

**Core Macros:**
- `if` — conditional (expands to `match`)
- `let` — local bindings (expands to nested `match`)
- `letfn` — mutually recursive local functions
- `fn` — multi-arity functions (expands to `fn*` + `match`)
- `defn` — named function definition
- `when`, `when-not` — single-branch conditional
- `cond` — multi-branch conditional
- `case` — value-based dispatch
- `->`, `->>` — threading macros
- `and`, `or` — short-circuit boolean
- `ns`, `in-ns` — namespace declaration

**Sequences:**
- `second`, `last`, `butlast`
- `map`, `filter`, `reduce`, `take`, `drop`
- `range`, `repeat`, `iterate`
- `concat`, `flatten`

**Collections:**
- `list`, `tuple`, `vector`, `hash-map`, `hash-set` — constructors from elements
- `vals`, `into`, `merge`
- `conj` (polymorphic add)
- `update`, `update-in`, `get-in`, `assoc-in`

**Predicates:**
- `seq?`, `coll?`, `seqable?`
- `pos?`, `neg?`, `zero?`, `even?`, `odd?`

**Other:**
- `identity`, `constantly`, `comp`, `partial`
- `juxt`, `some`, `every?`
- `format`, `println`
