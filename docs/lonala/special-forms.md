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

### Sequential Destructuring

Bindings support vector patterns for extracting values from collections:

```clojure
; Basic destructuring
(let [[a b] [1 2]]
  (+ a b))           ; => 3

; Rest binding with &
(let [[first & rest] [1 2 3 4]]
  rest)              ; => (2 3 4)

; :as binding to keep whole collection
(let [[a b :as all] [1 2 3]]
  all)               ; => [1 2 3]

; Ignoring elements with _
(let [[a _ c] [1 2 3]]
  c)                 ; => 3

; Nested destructuring
(let [[[x y] z] [[1 2] 3]]
  (+ x y z))         ; => 6

; Missing elements bind to nil
(let [[a b] [1]]
  b)                 ; => nil

; Destructuring nil
(let [[a] nil]
  a)                 ; => nil
```

### Associative Destructuring

Bindings also support map patterns for extracting values from maps:

```clojure
; :keys - extract by keyword keys
(let [{:keys [a b]} {:a 1 :b 2}]
  (+ a b))           ; => 3

; :strs - extract by string keys
(let [{:strs [name]} {"name" "Alice"}]
  name)              ; => "Alice"

; :syms - extract by symbol keys
(let [{:syms [x]} {'x 42}]
  x)                 ; => 42

; Explicit key binding
(let [{x :foo} {:foo 42}]
  x)                 ; => 42

; :or - default values for missing keys
(let [{:keys [a b] :or {b 0}} {:a 1}]
  (+ a b))           ; => 1

; :as - bind the whole map
(let [{:keys [a] :as m} {:a 1 :b 2}]
  m)                 ; => {:a 1 :b 2}

; Combined patterns
(let [{:keys [a b] :or {b 100} :as m} {:a 1}]
  [a b])             ; => [1 100]

; Missing keys bind to nil
(let [{:keys [a]} {}]
  a)                 ; => nil

; Destructuring nil map
(let [{:keys [a]} nil]
  a)                 ; => nil
```

**Note**: `:or` defaults apply only when the value is `nil`, not when it's `false`:

```clojure
(let [{:keys [a] :or {a true}} {:a false}]
  a)                 ; => false (not true)
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

### Parameter Destructuring

Parameters support sequential destructuring patterns, allowing direct extraction of values from collections:

```clojure
; Basic destructuring - extracts first two elements
((fn [[a b]] (+ a b)) [1 2])        ; => 3

; With rest binding - collects remaining elements
((fn [[x & xs]] xs) [1 2 3 4])      ; => (2 3 4)

; With :as binding - binds whole collection
((fn [[a :as all]] all) [1 2 3])    ; => [1 2 3]

; Ignoring elements with _
((fn [[a _ c]] c) [1 2 3])          ; => 3

; Nested destructuring
((fn [[[x y] z]] (+ x y z)) [[1 2] 3])  ; => 6
```

**Mixed parameters** - destructuring and simple params:

```clojure
((fn [[a b] c] (+ a b c)) [1 2] 3)  ; => 6
```

**Ignored parameters** - use `_` to ignore arguments:

```clojure
((fn [_ x] x) 1 2)          ; => 2
((fn [x & _] x) 1 2 3 4)    ; => 1
```

**Missing elements** bind to `nil`:

```clojure
((fn [[a b]] b) [1])        ; => nil
((fn [[a]] a) nil)          ; => nil
```

### Map Destructuring in Parameters

Parameters also support map patterns for extracting values from map arguments:

```clojure
; :keys - extract by keyword keys
((fn [{:keys [a b]}] (+ a b)) {:a 1 :b 2})  ; => 3

; :strs - extract by string keys
((fn [{:strs [name]}] name) {"name" "Alice"})  ; => "Alice"

; :syms - extract by symbol keys
((fn [{:syms [x]}] x) {'x 42})              ; => 42

; Explicit key binding
((fn [{x :foo}] x) {:foo 99})               ; => 99

; :or - default values
((fn [{:keys [a b] :or {b 100}}] (+ a b)) {:a 1})  ; => 101

; :as - bind whole map
((fn [{:keys [a] :as m}] m) {:a 1 :b 2})    ; => {:a 1 :b 2}
```

**Mixed parameters** - map destructuring with simple params:

```clojure
((fn [x {:keys [a b]}] (+ x a b)) 10 {:a 1 :b 2})  ; => 13
```

**Multi-arity with map destructuring**:

```clojure
(def f (fn ([{:keys [a]}] a)
           ([{:keys [a]} b] (+ a b))))
(f {:a 10})        ; => 10
(f {:a 10} 5)      ; => 15
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

## 6.8 Condition System *(Planned)*

Lonala provides a condition system inspired by Common Lisp that separates error detection from error handling. Unlike exceptions that immediately unwind the stack, conditions preserve full context and allow recovery.

### 6.8.1 `signal` *(Planned)*

Signals a condition without unwinding the stack.

**Syntax**: `(signal type data)`

**Parameters**:
- `type` — A keyword identifying the condition type
- `data` — A map of contextual information

**Behavior**:
- Searches for a handler established by `handler-bind`
- If handler found, calls it with the condition
- Handler can invoke a restart, re-signal, or return
- If no handler or handler returns, `signal` returns `nil` (non-fatal)

> **Note**: `signal` is for **non-fatal notifications** (warnings, info). For fatal errors,
> use `panic!` which terminates the process in production mode. See [Error Handling](error-handling.md)
> for the full distinction between `signal` and `panic!`.

```clojure
;; Signal a non-fatal condition (returns nil if unhandled)
(signal :file-not-found {:path "/etc/config"})

;; Signal with rich context
(signal :validation-failed {:field :email
                             :value "not-an-email"
                             :reason "Invalid format"})
```

### 6.8.2 `restart-case` *(Planned)*

Establishes restarts around a protected expression.

**Syntax**:
```clojure
(restart-case expr
  (restart-name [params*] description? body*)
  ...)
```

**Parameters**:
- `expr` — The expression to protect
- `restart-name` — Keyword naming the restart
- `params` — Parameters accepted by the restart
- `description` — Optional string describing the restart
- `body` — Code to execute if restart is invoked

**Returns**: Value of `expr` or the invoked restart's body

```clojure
(defn read-config [path]
  (restart-case
    (if (file-exists? path)
      (parse-config (slurp path))
      (signal :file-not-found {:path path}))

    (:retry []
      "Try reading the file again"
      (read-config path))

    (:use-default []
      "Use default configuration"
      default-config)

    (:use-value [config]
      "Provide a configuration value"
      config)))
```

### 6.8.3 `handler-bind` *(Planned)*

Establishes handlers for conditions.

**Syntax**:
```clojure
(handler-bind
  [condition-type handler-fn]*
  body*)
```

**Parameters**:
- `condition-type` — Keyword matching condition types
- `handler-fn` — Function called when matching condition signaled
- `body` — Expressions to evaluate with handlers active

**Handler Function**:
- Receives the condition as argument
- Can call `(invoke-restart restart-name args*)` to recover
- Can re-signal or signal a different condition
- If returns normally, condition is considered unhandled

```clojure
(handler-bind
  [:file-not-found
   (fn [condition]
     (if (= (:path condition) "/etc/critical.conf")
       (invoke-restart :use-default)
       (invoke-restart :retry)))]

  [:validation-failed
   (fn [condition]
     (log/warn "Validation failed" condition)
     (invoke-restart :use-value nil))]

  (start-application))
```

### 6.8.4 `invoke-restart` *(Planned)*

Invokes a restart established by `restart-case`.

**Syntax**: `(invoke-restart restart-name args*)`

**Parameters**:
- `restart-name` — Keyword naming the restart to invoke
- `args` — Arguments passed to the restart

**Behavior**: Transfers control to the restart, passing arguments.

```clojure
;; Invoke restart without arguments
(invoke-restart :use-default)

;; Invoke restart with arguments
(invoke-restart :use-value {:timeout 5000})

;; Invoke retry
(invoke-restart :retry)
```

## 6.9 Process Termination *(Planned)*

For truly unrecoverable errors—bugs, invariant violations, fatal hardware failures—Lonala provides `panic!`.

### 6.9.1 `panic!` *(Planned)*

Signals an unrecoverable condition.

**Syntax**: `(panic! message)` or `(panic! message data)`

**Parameters**:
- `message` — A string describing what went wrong
- `data` — Optional map of contextual information

**Behavior depends on debug mode** (see [Two-Mode Architecture](debugging.md#two-mode-architecture)):

| Mode | Behavior |
|------|----------|
| **Production** (default) | Terminates process immediately, supervisor restarts |
| **Debug** (attached) | Pauses process, presents debugger UI with restarts |

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

### 6.9.2 Production Mode Behavior

In production mode (no debugger attached):
- Process exits with reason `{:panic {:message msg :data data}}`
- Linked processes receive exit signal
- Supervisor applies restart strategy

```clojure
;; Supervisor restarts crashed workers
(def-supervisor my-workers
  :strategy :one-for-one
  :children
  [{:id :worker-1 :start worker/start}
   {:id :worker-2 :start worker/start}])

;; If worker-1 panics, supervisor restarts only worker-1
```

### 6.9.3 Debug Mode Behavior

In debug mode (debugger attached):
- Execution pauses at the panic point
- Debugger presents the error, stack, and available restarts
- User can inspect locals, evaluate expressions
- User chooses: continue (if possible), step, or crash

```
╭─ PROCESS BREAK ─────────────────────────────────────────────────╮
│ Panic: User must have an ID                                     │
│ Data: {:user {:name "Alice"}}                                   │
╰──────────────────────────────────────────────────────────────────╯

Restarts:
  [1] :abort      - Crash process, trigger supervisor restart
  [2] :continue   - Continue execution (may cause further errors)

proc-debug[0]> _
```

### 6.9.4 When to Use `panic!`

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

**Rule of thumb**: If the error indicates a bug or unrecoverable condition, use `panic!`. If the error is an expected possible outcome, return an error tuple.

See [Error Handling](error-handling.md) for the complete error handling philosophy.

### 6.9.5 `assert!` *(Planned)*

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

## 6.10 `receive` *(Planned)*

Pattern-matched message receive for process mailboxes.

**Syntax**: `(receive pattern1 result1 pattern2 result2 ... (after timeout timeout-result)?)`

**Parameters**:
- `pattern` — A pattern to match against incoming messages
- `result` — Expression to evaluate when pattern matches
- `timeout` — Optional timeout in milliseconds
- `timeout-result` — Expression to evaluate if timeout is reached

**Returns**: The value of the matched result expression, or the timeout-result

**Semantics**: Blocks the current process until a message arrives in its mailbox that matches one of the patterns. When a match is found, the corresponding result expression is evaluated with any pattern bindings in scope. If an `after` clause is provided and no matching message arrives within the timeout, the timeout-result is evaluated.

`receive` is a **special form** (not a function) because it involves pattern matching and blocking semantics handled by the compiler.

```clojure
;; Simple message handling
(receive
  {:type :greeting :text text}
    (print "Got greeting:" text)
  {:type :shutdown}
    (exit :normal))

;; With timeout
(receive
  {:type :response :data data}
    {:ok data}
  (after 5000
    {:error :timeout}))

;; In a server loop
(defn server-loop [state]
  (receive
    {:type :get :from pid}
      (do
        (send pid {:type :response :value state})
        (server-loop state))
    {:type :set :value new-state}
      (server-loop new-state)
    {:type :stop}
      :ok))
```

**Pattern Matching**: Patterns in `receive` support:
- Literal values: `:ok`, `42`, `"hello"`
- Binding symbols: `x`, `data`, `pid`
- Maps: `{:type :greeting :text text}`
- Vectors: `[first second & rest]`
- Nested patterns: `{:user {:name name :id id}}`

See [Concurrency](concurrency.md) (Planned) for more details on the process model and message passing.

---

