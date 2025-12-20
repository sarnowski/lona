# Macros
> **Status**: The macro system is implemented. `defmacro` (single and multi-arity), macro expansion at compile time, and introspection (`macro?`, `macroexpand-1`, `macroexpand`) are all working. `gensym` support is pending, which means hygienic macros require manual naming conventions for now.

Macros enable compile-time code transformation. They receive unevaluated code as data and return new code to be compiled.

## 11.1 Defining Macros

**Syntax**:
- Single arity: `(defmacro name [params*] body+)`
- Multi-arity: `(defmacro name ([params1*] body1+) ([params2*] body2+) ...)`

**Parameters**:
- `name` — A symbol naming the macro
- `params` — A vector of parameter symbols (may include `& rest`)
- `body` — One or more expressions forming the macro body

**Returns**: The symbol `name`

**Semantics**: Defines a macro that, when called during compilation, receives its arguments as unevaluated AST. The macro body should return transformed code (typically using quasiquote). Multi-arity macros dispatch based on argument count.

```clojure
; Single-arity macro
(defmacro unless [test body]
  `(if (not ~test) ~body nil))
; => unless

; Multi-arity macro
(defmacro when
  ([test] `(if ~test nil nil))
  ([test body] `(if ~test ~body nil))
  ([test body & more] `(if ~test (do ~body ~@more) nil)))
; => when
```

> **Note**: Macros are fully functional. They are defined with `defmacro`, stored in a persistent registry, and expanded at compile time using the VM-based macro expander. Introspection primitives (`macro?`, `macroexpand-1`, `macroexpand`) are implemented. The only missing piece is `gensym` for hygienic macro expansion.

## 11.2 Macro Introspection

```clojure
;; Check if a symbol names a macro
(macro? 'unless)  ; => true
(macro? 'if)      ; => false (special form)
(macro? 'foo)     ; => false (undefined)

;; macroexpand-1: expand exactly once
(macroexpand-1 '(unless false (print "hi")))
; => (when (not false) (print "hi"))   ; if unless expands to when

;; macroexpand: keep expanding while top-level is a macro
(macroexpand '(unless false (print "hi")))
; => (if (not false) (print "hi") nil) ; unless -> when -> if
```

**Key difference**: `macroexpand-1` performs a single expansion step. `macroexpand` iterates until the top-level form is no longer a macro call.

## 11.3 Common Macro Patterns

```clojure
; when - one-armed if
(defmacro when [test & body]
  `(if ~test (do ~@body) nil))

; defn - define named function
(defmacro defn [name params & body]
  `(def ~name (fn ~name ~params ~@body)))

; -> threading macro
(defmacro -> [x & forms]
  ...)
```

## 11.4 Error Handling Macros *(Planned)*

Standard library macros for ergonomic error handling with result tuples. These macros depend on pattern matching/destructuring which is not yet implemented.

### 11.4.1 `with` — Chaining Fallible Operations *(Planned)*

Sequences multiple operations that return result tuples, short-circuiting on the first error.

**Syntax**: `(with [bindings*] body)` or `(with [bindings*] body (else error-expr))`

**Parameters**:
- `bindings` — Pairs of `pattern expression` where expression returns a result tuple
- `body` — Expression evaluated if all bindings succeed
- `error-expr` — Expression evaluated if any binding fails (optional)

```clojure
;; Basic usage
(with [{:ok user}     (fetch-user id)
       {:ok profile}  (fetch-profile user)
       {:ok settings} (load-settings profile)]
  {:ok (build-context user profile settings)})
; If any step returns {:error reason}, that error is returned

;; With else clause for error transformation
(with [{:ok config}   (read-config path)
       {:ok parsed}   (parse-config config)
       {:ok validated} (validate-config parsed)]
  {:ok validated}
  (else err
    {:error {:phase :config-load :cause err}}))
```

**Expansion**:
```clojure
(with [{:ok a} (foo)
       {:ok b} (bar a)]
  {:ok (baz a b)})

;; Expands to:
(case (foo)
  {:ok a} (case (bar a)
            {:ok b} {:ok (baz a b)}
            err     err)
  err     err)
```

### 11.4.2 `if-ok` — Conditional on Success *(Planned)*

**Syntax**: `(if-ok [binding expr] then else?)`

```clojure
(if-ok [{:ok user} (fetch-user id)]
  (greet user)
  (show-login-form))

;; With destructuring
(if-ok [{:ok {:keys [name email]}} (fetch-user id)]
  (str "Hello " name)
  "Guest")
```

### 11.4.3 `when-ok` — Execute on Success *(Planned)*

**Syntax**: `(when-ok [binding expr] body*)`

```clojure
(when-ok [{:ok config} (load-config)]
  (apply-config config)
  (log "Config loaded"))
; Returns nil if load-config fails
```

### 11.4.4 `ok->` — Threading with Short-Circuit *(Planned)*

Threads value through functions that return result tuples, stopping on first error.

**Syntax**: `(ok-> expr forms*)`

```clojure
(ok-> user-id
      fetch-user        ; {:ok user} or {:error ...}
      validate-user     ; {:ok user} or {:error ...}
      activate-user)    ; {:ok user} or {:error ...}

;; Equivalent to:
(and-then (and-then (fetch-user user-id) validate-user) activate-user)
```

### 11.4.5 `ok->>` — Threading Last with Short-Circuit *(Planned)*

Like `ok->` but threads as last argument.

```clojure
(ok->> items
       (filter valid?)      ; returns {:ok filtered} or {:error ...}
       (map transform)      ; returns {:ok mapped} or {:error ...}
       (save-all db))       ; returns {:ok result} or {:error ...}
```

---

