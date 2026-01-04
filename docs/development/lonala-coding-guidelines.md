# Lonala Coding Guidelines

Style conventions for writing clear, consistent Lonala code.

---

## File Structure

### License Header

Every source file begins with the SPDX license identifier and copyright:

```clojure
;; SPDX-License-Identifier: GPL-3.0-or-later
;; Copyright 2026 Tobias Sarnowski
```

### Section Organization

Use visual separators to organize code into logical sections:

```clojure
;; ═══════════════════════════════════════════════════════════════════════════
;; Module Title — Brief description
;; ═══════════════════════════════════════════════════════════════════════════

;; ───────────────────────────────────────────────────────────────────────────
;; Section Name
;; ───────────────────────────────────────────────────────────────────────────
```

### Blank Lines

- Single blank line between top-level forms
- No blank lines inside function bodies (except to separate `let` binding groups)
- End files with a newline

---

## Formatting

### Indentation

- Use 2 spaces for indentation
- Never use tabs
- Align function arguments vertically when spanning multiple lines

```clojure
;; Good
(defn process-data
  [input options]
  (let [parsed (parse input)
        validated (validate parsed options)]
    (transform validated)))

;; Good - aligned arguments
(some-function arg1
               arg2
               arg3)
```

### Line Length

- Target 80 characters
- Maximum 100 characters for readability

### Parentheses

Gather closing parentheses on a single line:

```clojure
;; Good
(defn factorial [n]
  (if (<= n 1)
    1
    (* n (factorial (- n 1)))))

;; Bad
(defn factorial [n]
  (if (<= n 1)
    1
    (* n (factorial (- n 1)))
  )
)
```

---

## Naming Conventions

| Pattern | Convention | Example |
|---------|------------|---------|
| Functions/vars | `lisp-case` | `process-request`, `user-id` |
| Predicates | Suffix with `?` | `empty?`, `valid-user?` |
| Side effects | Suffix with `!` | `reset!`, `send!` |
| Dynamic vars | Wrap in `*earmuffs*` | `*ns*`, `*debug-mode*` |
| Private | Use `defn-` or `^:private` | `defn- helper` |
| Constants | No special notation | `max-retries`, `default-timeout` |

### Predicates

Functions returning boolean should end with `?`:

```clojure
(defn valid-email? [s]
  (and (string? s)
       (contains-char? s \@)))
```

### Destructive Operations

Functions with side effects should end with `!`:

```clojure
(defn save-config! [config]
  (write-file! config-path (serialize config)))
```

---

## Docstrings

### Placement

Docstrings go after the function name, before the parameter vector:

```clojure
(defn frobnicate
  "Transforms `x` according to the frobnication algorithm.

  Returns a tuple of `[:ok result]` or `[:error reason]`."
  [x]
  ...)
```

### Content Guidelines

1. **First line**: Complete, capitalized sentence describing the function
2. **Arguments**: Wrap in backticks: `` `x` ``, `` `options` ``
3. **Return values**: Document the return type/structure
4. **Multi-line**: Indent continuation by 2 spaces
5. **No surrounding whitespace**: Don't start or end with blank lines

### Examples

```clojure
;; Good - concise, describes behavior
(defn partition
  "Splits `coll` into groups of `n` elements.

  Returns a list of tuples. The final group may have fewer
  than `n` elements if `coll` is not evenly divisible."
  [n coll]
  ...)

;; Good - documents return structure
(defn parse-config
  "Parses configuration from `path`.

  Returns `[:ok config-map]` on success or
  `[:error reason]` on failure."
  [path]
  ...)
```

---

## Code Style

### Prefer Functions Over Macros

Only use macros when you need:
- Control over evaluation (short-circuit, delayed evaluation)
- New syntactic forms
- Compile-time code generation

```clojure
;; Use a function when possible
(defn double [x]
  (* x 2))

;; Use a macro when controlling evaluation
(defmacro when-debug [& body]
  `(when *debug-mode*
     ~@body))
```

### Keep Functions Short

- Target 5-10 lines of code
- Extract helper functions for complex logic
- Each function should do one thing

### Destructuring

Use destructuring for clarity, but don't nest too deeply:

```clojure
;; Good
(defn process-request [{:keys [method path headers]}]
  ...)

;; Good
(defn handle-response [[status body]]
  ...)

;; Avoid - too deep
(defn bad-example [{{:keys [x y]} :coords {:keys [w h]} :size}]
  ...)
```

### Threading Macros

Use `->` and `->>` to improve readability of data transformations:

```clojure
;; Good - clear data flow
(-> request
    parse-body
    validate
    process
    format-response)

;; Avoid - deeply nested
(format-response (process (validate (parse-body request))))
```

---

## Error Handling

### Use Tuple Returns

Return `[:ok result]` or `[:error reason]` for recoverable errors:

```clojure
(defn parse-int [s]
  (if (valid-integer-string? s)
    [:ok (string->int s)]
    [:error :invalid-format]))
```

### Let It Crash

For unexpected errors, let the process crash. Don't catch errors you can't handle meaningfully:

```clojure
;; Good - let supervisor handle failures
(defn process-message [msg]
  (let [[_ data] msg]
    (do-work data)))

;; Avoid - hiding errors
(defn process-message [msg]
  (match (try-parse msg)
    [:error _] nil  ; silently dropping errors
    [:ok data] (do-work data)))
```

---

## Comments

### When to Comment

- Explain **why**, not **what**
- Document non-obvious design decisions
- Add section headers for organization

```clojure
;; Good - explains why
;; Using bit manipulation here because this is called millions
;; of times per second in the hot path
(defn fast-hash [x]
  (bit-xor x (bit-shr x 16)))

;; Bad - states the obvious
;; Adds one to x
(defn inc [x]
  (+ x 1))
```

### Comment Syntax

```clojure
;; Single-line comment

;; Multi-line comments use
;; multiple single-line comments

#_ (this form is ignored by the reader)

(comment
  ;; Use comment blocks for examples and scratch code
  (example-usage 1 2 3))
```

---

## Summary

| Guideline | Rule |
|-----------|------|
| Indentation | 2 spaces, no tabs |
| Line length | 80 chars target, 100 max |
| Blank lines | Single between top-level forms |
| Naming | `lisp-case`, `?` for predicates, `!` for side effects |
| Docstrings | First line is complete sentence, backtick args |
| Functions | Short (5-10 LOC), single responsibility |
| Errors | Tuple returns for recoverable, crash for unexpected |
| Comments | Explain why, not what |
