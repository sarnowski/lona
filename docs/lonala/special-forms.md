# Special Forms
Special forms are fundamental language constructs with evaluation rules that differ from normal function calls. They cannot be implemented as functions.

## 6.1 `def`

Binds a value to a global variable.

**Syntax**: `(def name value)`

**Parameters**:
- `name` — A symbol naming the variable
- `value` — An expression to evaluate and bind

**Returns**: The symbol `name`

**Semantics**: Evaluates `value` and binds the result to `name` in the global environment. If `name` is already defined, it is rebound to the new value.

```clojure
(def x 42)          ; => x
x                   ; => 42

(def greeting "Hello, World!")
greeting            ; => "Hello, World!"

(def square (fn [n] (* n n)))
(square 5)          ; => 25
```

## 6.2 `let`

Creates local bindings for a body of expressions.

**Syntax**: `(let [bindings*] body*)`

**Parameters**:
- `bindings` — A vector of alternating symbols and values: `[name1 val1 name2 val2 ...]`
- `body` — Zero or more expressions to evaluate

**Returns**: The value of the last body expression, or `nil` if body is empty

**Semantics**:
1. Bindings are evaluated left-to-right
2. Each binding can refer to previously bound names
3. Body expressions are evaluated with all bindings in scope
4. Bindings are local to the `let` form

```clojure
(let [x 10]
  x)                ; => 10

(let [x 10
      y 20]
  (+ x y))          ; => 30

(let [x 10
      y (* x 2)]    ; y can use x
  y)                ; => 20

(let [x 1]
  (let [x 2]        ; inner x shadows outer
    x))             ; => 2
```

## 6.3 `if`

Conditional branching.

**Syntax**: `(if test then else?)`

**Parameters**:
- `test` — Expression to evaluate for truthiness
- `then` — Expression to evaluate if test is truthy
- `else` — Expression to evaluate if test is falsy (optional, defaults to `nil`)

**Returns**: The value of `then` or `else` branch

**Semantics**: Evaluates `test`. If the result is truthy (not `nil` or `false`), evaluates and returns `then`. Otherwise, evaluates and returns `else` (or `nil` if `else` is omitted).

```clojure
(if true "yes" "no")      ; => "yes"
(if false "yes" "no")     ; => "no"
(if nil "yes" "no")       ; => "no"
(if 0 "yes" "no")         ; => "yes" (0 is truthy)

(if (> 5 3)
  "five is greater"
  "three is greater")     ; => "five is greater"

(if false "yes")          ; => nil (no else branch)
```

## 6.4 `do`

Sequential execution of multiple expressions.

**Syntax**: `(do exprs*)`

**Parameters**:
- `exprs` — Zero or more expressions

**Returns**: The value of the last expression, or `nil` if empty

**Semantics**: Evaluates each expression in order, returning the value of the final expression. Earlier expressions are evaluated for their side effects.

```clojure
(do)                      ; => nil

(do 1 2 3)                ; => 3

(do
  (print "first")
  (print "second")
  "done")                 ; prints, then => "done"
```

## 6.5 `fn`

Creates a function.

**Syntax**: `(fn name? [params*] body*)`

**Parameters**:
- `name` — Optional symbol for recursion and debugging
- `params` — Vector of parameter symbols
- `body` — Zero or more expressions forming the function body

**Returns**: A function value

**Semantics**: Creates a new function that, when called:
1. Binds arguments to parameter names
2. Evaluates body expressions in order
3. Returns the value of the last expression

```clojure
; Anonymous function
(fn [x] (* x x))

; Named function (for recursion)
(fn factorial [n]
  (if (<= n 1)
    1
    (* n (factorial (- n 1)))))

; Multiple body expressions
(fn [x]
  (print "computing...")
  (* x x))

; No parameters
(fn [] 42)

; Multiple parameters
(fn [a b c] (+ a b c))
```

**Calling functions**:

```clojure
((fn [x] (* x x)) 5)      ; => 25

(def square (fn [x] (* x x)))
(square 5)                 ; => 25
```

## 6.6 `quote`

Returns its argument unevaluated.

**Syntax**: `(quote form)`

**Parameters**:
- `form` — Any expression

**Returns**: The form itself, as data

**Semantics**: Prevents evaluation of the form. Lists become list data, symbols become symbol data.

```clojure
(quote foo)               ; => foo (the symbol)
(quote (+ 1 2))           ; => (+ 1 2) (the list)
(quote [1 2 3])           ; => [1 2 3]

; Shorthand with reader macro
'foo                      ; => foo
'(+ 1 2)                  ; => (+ 1 2)
```

## 6.7 `syntax-quote`

Template quoting with unquote support.

**Syntax**: `` `form `` or `(syntax-quote form)`

**Parameters**:
- `form` — A template expression

**Returns**: The form with unquoted parts evaluated

**Semantics**: Like `quote`, but allows selective evaluation within the template using `~` (unquote) and `~@` (unquote-splicing).

```clojure
`(1 2 3)                  ; => (1 2 3)

(let [x 10]
  `(1 ~x 3))              ; => (1 10 3)

(let [nums [2 3 4]]
  `(1 ~@nums 5))          ; => (1 2 3 4 5)
```

See [Reader Macros](reader-macros.md) for details on unquote operators.

## 6.8 Process Termination *(Planned)*

For truly unrecoverable errors—bugs, invariant violations, fatal hardware failures—Lonala provides `panic!` to terminate the current process immediately.

### 6.8.1 `panic!` *(Planned)*

Terminates the current process with an error reason.

**Syntax**: `(panic! message)` or `(panic! message data)`

**Parameters**:
- `message` — A string describing what went wrong
- `data` — Optional map of contextual information

**Behavior**:
- Immediately terminates the current process
- The process exits with reason `{:panic {:message msg :data data}}`
- **Cannot be caught** — this is intentional
- Linked processes and supervisors receive the exit signal

```clojure
;; Invariant violation (bug)
(defn process-user [user]
  (when (nil? (:id user))
    (panic! "User must have an ID" {:user user}))
  ...)

;; Unrecoverable hardware
(defn reset-device [dev]
  (write-register dev RESET 1)
  (wait-ms 100)
  (when (not (device-ready? dev))
    (panic! "Device failed to reset" {:device (:name dev)})))

;; Corruption detected
(defn load-critical-data [path]
  (let [data (read-file! path)
        checksum (compute-checksum data)]
    (when (not= checksum (:expected-checksum data))
      (panic! "Critical data corrupted" {:path path}))
    data))
```

### 6.8.2 When to Use `panic!`

| Situation | Use `panic!`? | Instead |
|-----------|---------------|---------|
| File not found | No | Return `{:error :not-found}` |
| Invalid user input | No | Return `{:error {:validation ...}}` |
| Network timeout | No | Return `{:error :timeout}` |
| Resource exhausted | No | Return `{:error :resource-exhausted}` |
| Nil where value required (bug) | Yes | — |
| Invariant violation (bug) | Yes | — |
| Data corruption | Yes | — |
| Hardware unrecoverable | Yes | — |

**Rule of thumb**: If the error indicates a bug in the code or an unrecoverable external condition, use `panic!`. If the error is an expected possible outcome, return an error tuple.

### 6.8.3 Recovery via Supervision

Processes terminated by `panic!` are restarted by their supervisor:

```clojure
;; Supervisor restarts crashed workers
(def-supervisor my-workers
  :strategy :one-for-one
  :children
  [{:id :worker-1 :start worker/start}
   {:id :worker-2 :start worker/start}])

;; If worker-1 panics, supervisor restarts only worker-1
;; Other workers continue unaffected
```

See [Concurrency](concurrency.md) for supervision details.

### 6.8.4 `assert!` *(Planned)*

Convenience macro for invariant checking.

**Syntax**: `(assert! test)` or `(assert! test message)`

**Expands to**:
```clojure
(when (not test)
  (panic! (or message "Assertion failed") {:expr 'test}))
```

**Examples**:
```clojure
(assert! (> n 0))
(assert! (valid-state? state) "Invalid state transition")
```

---

