# lona.core

Core language intrinsics. These are native functions implemented in the VM.

---

## Namespace and Vars

### `*ns*`

Process-local var holding current namespace. Each process has its own `*ns*` value,
inherited from the parent at spawn time.

```clojure
*ns*  ; → current namespace

;; Change namespace for current process
(def *ns* (find-ns 'my.namespace))
```

### `create-ns`

Create a namespace.

```clojure
(create-ns sym)  ; → namespace
```

### `find-ns`

Find namespace by symbol.

```clojure
(find-ns sym)  ; → namespace or nil
```

### `ns-name`

Get namespace's symbol name.

```clojure
(ns-name ns)  ; → symbol
```

### `ns-map`

Get namespace's var bindings.

```clojure
(ns-map ns)  ; → map of symbol → var
```

### `intern`

Intern a var in namespace.

```clojure
(intern ns sym)        ; → var
(intern ns sym val)    ; → var (with initial value)
```

### `refer`

Add var references from another namespace.

```clojure
(refer ns)  ; → nil
```

### `alias`

Create namespace alias.

```clojure
(alias alias-sym ns-sym)  ; → nil
```

### `var`

Get var object by name. The `var` special form looks up a var by symbol name
and returns the var object (not its value).

```clojure
(var sym)   ; returns var object for sym
#'sym       ; reader syntax, equivalent to (var sym)
#'ns/sym    ; qualified var lookup
```

Note: `var` is a special form - the symbol argument is not evaluated.

### `var-get`

Get value of var.

```clojure
(var-get v)  ; → value
```

### `meta`

Get metadata of object.

```clojure
(meta obj)  ; → map or nil
```

### `with-meta`

Return object with new metadata.

```clojure
(with-meta obj meta-map)  ; → obj with metadata
```

---

## Evaluation

### `eval`

Evaluate form.

```clojure
(eval form)  ; → result
```

### `read-string`

Parse string to form.

```clojure
(read-string s)  ; → form
```

### `macroexpand`

Fully expand macros in form.

```clojure
(macroexpand form)  ; → expanded form
```

### `macroexpand-1`

Expand macros once.

```clojure
(macroexpand-1 form)  ; → expanded form
```

### `gensym`

Generate unique symbol.

```clojure
(gensym)         ; → symbol
(gensym prefix)  ; → symbol with prefix
```

### `load-file`

Load and evaluate file.

```clojure
(load-file path)  ; → last value
```

---

## Arithmetic

### `+`

Addition.

```clojure
(+ a b ...)  ; → sum
```

### `-`

Subtraction.

```clojure
(- a)        ; → negation
(- a b ...)  ; → difference
```

### `*`

Multiplication.

```clojure
(* a b ...)  ; → product
```

### `/`

Division.

```clojure
(/ a b ...)  ; → quotient
```

### `mod`

Modulus (sign follows divisor).

```clojure
(mod a b)  ; → remainder
```

### `quot`

Integer quotient (truncates toward zero).

```clojure
(quot a b)  ; → integer
```

### `rem`

Remainder (sign follows dividend).

```clojure
(rem a b)  ; → remainder
```

### `checked-add`

Addition with overflow check.

```clojure
(checked-add a b)  ; → [:ok result] or [:error :overflow]
```

### `saturating-add`

Addition clamped to type bounds.

```clojure
(saturating-add a b)  ; → result (clamped)
```

---

## Ratio

### `numerator`

Get numerator of ratio.

```clojure
(numerator r)  ; → integer
```

### `denominator`

Get denominator of ratio.

```clojure
(denominator r)  ; → integer
```

---

## Comparison

### `=`

Equality (value-based).

```clojure
(= a b ...)  ; → boolean
```

### `<`

Less than.

```clojure
(< a b ...)  ; → boolean
```

### `>`

Greater than.

```clojure
(> a b ...)  ; → boolean
```

### `<=`

Less than or equal.

```clojure
(<= a b ...)  ; → boolean
```

### `>=`

Greater than or equal.

```clojure
(>= a b ...)  ; → boolean
```

### `not=`

Not equal.

```clojure
(not= a b ...)  ; → boolean
```

### `identical?`

Reference identity.

```clojure
(identical? a b)  ; → boolean
```

---

## Boolean

### `not`

Logical negation.

```clojure
(not x)  ; → boolean
```

---

## Collections

### `prepend`

Add element to front of list.

```clojure
(prepend list elem)  ; → list
```

### `append`

Add element to end of vector or tuple.

```clojure
(append coll elem)  ; → coll
```

### `put`

Associate key-value in map.

```clojure
(put map key val)  ; → map
```

### `set-add`

Add element to set.

```clojure
(set-add set elem)  ; → set
```

### `dissoc`

Remove key from map.

```clojure
(dissoc map key)  ; → map
```

### `set-remove`

Remove element from set.

```clojure
(set-remove set elem)  ; → set
```

### `count`

Number of elements.

```clojure
(count coll)  ; → integer
```

### `nth`

Get element by index.

```clojure
(nth coll idx)            ; → element (or error)
(nth coll idx not-found)  ; → element or not-found
```

### `get`

Get value by key.

```clojure
(get coll key)            ; → value or nil
(get coll key not-found)  ; → value or not-found
```

### `keys`

Get map keys.

```clojure
(keys map)  ; → list of keys
```

### `vals`

Get map values.

```clojure
(vals map)  ; → list of values
```

---

## Sequences

These intrinsics are **polymorphic** — they work on any collection type (list, tuple,
vector, map, set). This enables generic functions like `map` and `filter` to be derived
once and work on all collections.

### `first`

First element of any collection.

```clojure
(first coll)  ; → element or nil

(first '(1 2 3))      ; → 1
(first [1 2 3])       ; → 1 (tuple)
(first {1 2 3})       ; → 1 (vector)
(first %{:a 1 :b 2})  ; → [:a 1] (key-value tuple)
(first #{3 1 2})      ; → <some element> (order unspecified)
(first nil)           ; → nil
(first '())           ; → nil
```

### `rest`

Remaining elements after first. **Always returns a list**, regardless of input type.

```clojure
(rest coll)  ; → list (empty list if coll has 0-1 elements)

(rest '(1 2 3))      ; → (2 3)
(rest [1 2 3])       ; → (2 3) - list, not tuple
(rest {1 2 3})       ; → (2 3) - list, not vector
(rest %{:a 1 :b 2})  ; → ([:b 2]) - list of tuples
(rest '(1))          ; → ()
(rest nil)           ; → ()
```

### `empty?`

True if collection has no elements.

```clojure
(empty? coll)  ; → boolean

(empty? '())   ; → true
(empty? nil)   ; → true
(empty? [])    ; → true
(empty? '(1))  ; → false
```

---

## Type Predicates

```clojure
(nil? x)          ; → boolean
(boolean? x)      ; → boolean
(true? x)         ; → boolean
(false? x)        ; → boolean
(number? x)       ; → boolean
(integer? x)      ; → boolean
(float? x)        ; → boolean
(ratio? x)        ; → boolean
(string? x)       ; → boolean
(char? x)         ; → boolean
(symbol? x)       ; → boolean
(keyword? x)      ; → boolean
(fn? x)           ; → boolean
(var? x)          ; → boolean
(list? x)         ; → boolean
(tuple? x)        ; → boolean
(vector? x)       ; → boolean
(map? x)          ; → boolean
(set? x)          ; → boolean
(binary? x)       ; → boolean
(bytebuf? x)      ; → boolean
(paddr? x)        ; → boolean
(vaddr? x)        ; → boolean
(realm-id? x)     ; → boolean
(pid? x)          ; → boolean
(ref? x)          ; → boolean
(cap? x)          ; → boolean
(msg-info? x)     ; → boolean
(notification? x) ; → boolean
(region? x)       ; → boolean
(dma-buffer? x)   ; → boolean
(ring? x)         ; → boolean
```

### `type`

Get type keyword.

```clojure
(type x)  ; → :nil :boolean :integer :string :list :tuple :map ...
```

---

## Type Coercion

Fixed-width integer conversion:

```clojure
(u8 x)   ; → u8
(u16 x)  ; → u16
(u32 x)  ; → u32
(u64 x)  ; → u64
(i8 x)   ; → i8
(i16 x)  ; → i16
(i32 x)  ; → i32
(i64 x)  ; → i64
```

Character conversion:

```clojure
(char x)  ; → character (from integer code point)
```

---

## Strings

### `str`

Concatenate to string.

```clojure
(str a b ...)  ; → string
```

### `subs`

Substring.

```clojure
(subs s start)      ; → substring from start
(subs s start end)  ; → substring from start to end
```

### `symbol`

Create symbol.

```clojure
(symbol name)       ; → symbol
(symbol ns name)    ; → qualified symbol
```

### `keyword`

Create keyword.

```clojure
(keyword name)      ; → keyword
(keyword ns name)   ; → qualified keyword
```

### `name`

Get name part of symbol/keyword.

```clojure
(name sym-or-kw)  ; → string
```

### `namespace`

Get namespace part of symbol/keyword.

```clojure
(namespace sym-or-kw)  ; → string or nil
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
(make-ref)  ; → ref
```

---

## Function Application

### `apply`

Apply function to args.

```clojure
(apply f args)           ; → result
(apply f a b ... args)   ; → result
```

---

## Bitwise Operations

### Basic

```clojure
(bit-and a b ...)   ; Bitwise AND
(bit-or a b ...)    ; Bitwise OR
(bit-xor a b ...)   ; Bitwise XOR
(bit-not a)         ; Bitwise NOT
```

### Shifts

```clojure
(bit-shl a n)   ; Shift left
(bit-shr a n)   ; Logical shift right (zero-fill)
(bit-sar a n)   ; Arithmetic shift right (sign-extend)
```

### Rotations

```clojure
(bit-rol a n width)   ; Rotate left
(bit-ror a n width)   ; Rotate right
```

### Single-Bit

```clojure
(bit-test a n)    ; Test if bit n is set
(bit-set a n)     ; Set bit n
(bit-clear a n)   ; Clear bit n
(bit-flip a n)    ; Toggle bit n
```

### Bit Fields

```clojure
(bit-field a start len)           ; Extract bits [start, start+len)
(bit-field-set a start len val)   ; Insert val into bit field
```

### Bit Counting

```clojure
(bit-count a)        ; Population count (number of 1 bits)
(leading-zeros a)    ; Count leading zeros
(trailing-zeros a)   ; Count trailing zeros
(leading-ones a)     ; Count leading ones
(trailing-ones a)    ; Count trailing ones
```

### Byte Order

```clojure
(byte-reverse16 a)   ; Reverse bytes in u16
(byte-reverse32 a)   ; Reverse bytes in u32
(byte-reverse64 a)   ; Reverse bytes in u64
```

### Masks

```clojure
(mask-bits n)           ; Mask with n low bits set
(mask-range start end)  ; Mask for bits [start, end)
```

---

## Binary

### `binary`

Create binary from byte sequence.

```clojure
(binary bytes)  ; → binary
```

### `binary-size`

Size in bytes.

```clojure
(binary-size bin)  ; → u64
```

### `binary-ref`

Get byte at offset.

```clojure
(binary-ref bin offset)  ; → u8
```

### `binary-slice`

Extract slice.

```clojure
(binary-slice bin start len)  ; → binary
```

### `binary-concat`

Concatenate binaries.

```clojure
(binary-concat bin1 bin2)  ; → binary
```

### `binary->string`

Decode to string.

```clojure
(binary->string bin)            ; → string (UTF-8)
(binary->string bin encoding)   ; → string
```

Encodings: `:utf-8` (default), `:ascii`, `:latin1`

### `string->binary`

Encode string.

```clojure
(string->binary s)            ; → binary (UTF-8)
(string->binary s encoding)   ; → binary
```

Encodings: `:utf-8` (default), `:ascii`, `:latin1`

---

## Bytebuf

### `bytebuf-alloc`

Allocate zeroed buffer.

```clojure
(bytebuf-alloc size)  ; → bytebuf
```

### `bytebuf-alloc-unsafe`

Allocate uninitialized buffer.

```clojure
(bytebuf-alloc-unsafe size)  ; → bytebuf
```

### `bytebuf-size`

Buffer size.

```clojure
(bytebuf-size buf)  ; → u64
```

### `bytebuf->binary`

Convert to immutable binary.

```clojure
(bytebuf->binary buf)                 ; → binary
(bytebuf->binary buf offset len)      ; → binary (slice)
```

### Read Operations

Native endianness:

```clojure
(bytebuf-read8 buf offset)    ; → u8
(bytebuf-read16 buf offset)   ; → u16
(bytebuf-read32 buf offset)   ; → u32
(bytebuf-read64 buf offset)   ; → u64
```

Little-endian:

```clojure
(bytebuf-read16-le buf offset)   ; → u16
(bytebuf-read32-le buf offset)   ; → u32
(bytebuf-read64-le buf offset)   ; → u64
```

Big-endian:

```clojure
(bytebuf-read16-be buf offset)   ; → u16
(bytebuf-read32-be buf offset)   ; → u32
(bytebuf-read64-be buf offset)   ; → u64
```

Signed:

```clojure
(bytebuf-read-i8 buf offset)       ; → i8
(bytebuf-read-i16-le buf offset)   ; → i16
(bytebuf-read-i16-be buf offset)   ; → i16
(bytebuf-read-i32-le buf offset)   ; → i32
(bytebuf-read-i32-be buf offset)   ; → i32
(bytebuf-read-i64-le buf offset)   ; → i64
(bytebuf-read-i64-be buf offset)   ; → i64
```

### Write Operations

Native endianness:

```clojure
(bytebuf-write8! buf offset val)    ; → :ok
(bytebuf-write16! buf offset val)   ; → :ok
(bytebuf-write32! buf offset val)   ; → :ok
(bytebuf-write64! buf offset val)   ; → :ok
```

Little-endian:

```clojure
(bytebuf-write16-le! buf offset val)   ; → :ok
(bytebuf-write32-le! buf offset val)   ; → :ok
(bytebuf-write64-le! buf offset val)   ; → :ok
```

Big-endian:

```clojure
(bytebuf-write16-be! buf offset val)   ; → :ok
(bytebuf-write32-be! buf offset val)   ; → :ok
(bytebuf-write64-be! buf offset val)   ; → :ok
```

### Bulk Operations

```clojure
(bytebuf-copy! dst dst-off src src-off len)   ; → :ok
(bytebuf-fill! buf offset len val)            ; → :ok
```

---

## Endianness

```clojure
(native-endian)        ; → :little or :big

(native->be16 val)     ; Convert native to big-endian
(native->be32 val)
(native->be64 val)
(native->le16 val)     ; Convert native to little-endian
(native->le32 val)
(native->le64 val)
(be->native16 val)     ; Convert big-endian to native
(be->native32 val)
(be->native64 val)
(le->native16 val)     ; Convert little-endian to native
(le->native32 val)
(le->native64 val)
```

---

## Addresses

### Physical Address

```clojure
(paddr val)                      ; Create paddr from u64
(paddr+ addr offset)             ; Add offset → paddr
(paddr- addr1 addr2)             ; Difference → u64
(paddr->u64 addr)                ; Extract as u64
(paddr-align addr alignment)     ; Round up
(paddr-align-down addr alignment); Round down
(paddr-aligned? addr alignment)  ; Check alignment
(paddr= addr1 addr2)             ; Equality
(paddr< addr1 addr2)             ; Less than
```

### Virtual Address

```clojure
(vaddr val)                      ; Create vaddr from u64
(vaddr+ addr offset)             ; Add offset → vaddr
(vaddr- addr1 addr2)             ; Difference → u64
(vaddr->u64 addr)                ; Extract as u64
(vaddr-align addr alignment)     ; Round up
(vaddr-align-down addr alignment); Round down
(vaddr-aligned? addr alignment)  ; Check alignment
(vaddr= addr1 addr2)             ; Equality
(vaddr< addr1 addr2)             ; Less than
```

---

## PID

```clojure
(pid realm-id local-id)   ; Create PID
(pid-realm p)             ; Get realm ID
(pid-local p)             ; Get local ID
(pid= p1 p2)              ; Equality
```

---

## Capabilities

### Type Predicates

```clojure
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
(cap-type cap)              ; → :tcb :endpoint :frame ...
(cap-rights cap)            ; → #{:read :write :grant ...}
(cap-has-right? cap right)  ; → boolean
```

---

## Message Info

```clojure
(msg-info label length caps)   ; Create msg-info
(msg-info-label mi)            ; Get label
(msg-info-length mi)           ; Get length
(msg-info-caps mi)             ; Get caps count
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
- `contains?`

**Predicates:**
- `seq?`, `coll?`, `seqable?`
- `pos?`, `neg?`, `zero?`, `even?`, `odd?`

**Other:**
- `identity`, `constantly`, `comp`, `partial`
- `juxt`, `some`, `every?`
- `format`, `println`
