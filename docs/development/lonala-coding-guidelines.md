# Lonala Coding Guidelines

This document defines coding guidelines and best practices for developing in Lonala, Lona's Clojure-inspired programming language. These guidelines ensure consistency, readability, and maintainability across the codebase.

---

## Overview

Lonala is a Lisp dialect inspired by Clojure and Erlang, designed for programming Lona's userspace. As a Lisp, it follows established conventions from the Clojure community while adapting to the unique requirements of an operating system environment.

| Aspect | Lonala Approach |
|--------|-----------------|
| Syntax | S-expressions (Lisp) |
| Naming | kebab-case (lisp-case) |
| Indentation | 2 spaces |
| Documentation | Docstrings with Markdown |
| Paradigm | Functional, immutable-first |

---

## File Headers

Every Lonala source file must begin with a comment block containing the SPDX license identifier:

```clojure
;; SPDX-License-Identifier: GPL-3.0-or-later
;; Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>
```

After the license header, include a module-level documentation comment describing the file's purpose:

```clojure
;; SPDX-License-Identifier: GPL-3.0-or-later
;; Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

;; Lona Core Standard Library
;;
;; This file defines the core macros that form the foundation of Lonala
;; programming. These macros are loaded automatically at runtime startup.
```

---

## Naming Conventions

### General Rules

| Category | Convention | Example |
|----------|------------|---------|
| Functions | kebab-case | `process-message`, `send-data` |
| Variables | kebab-case | `user-count`, `buffer-size` |
| Macros | kebab-case | `defn`, `when-let` |
| Constants | kebab-case | `max-connections`, `default-timeout` |
| Predicates | Ends with `?` | `empty?`, `valid-email?` |
| Unsafe/Mutating | Ends with `!` | `reset!`, `swap!` |
| Private | Prefix with `-` | `-internal-helper` |
| Unused bindings | Single `_` or `_name` | `_`, `_unused` |

### Naming Philosophy

- **Descriptive over terse**: Choose clarity over brevity
- **Domain vocabulary**: Use terms from the problem domain
- **Verb-first for actions**: `send-message`, `create-process`
- **Noun-first for data**: `user-list`, `config-map`

```clojure
;; Good: descriptive, domain-appropriate
(defn calculate-process-priority [process scheduler-state]
  ...)

;; Bad: cryptic abbreviations
(defn calc-prio [p s]
  ...)
```

---

## Indentation

### Basic Rules

- **Use 2 spaces** for indentation (never tabs)
- **Body forms** are indented 2 spaces from the opening form
- **Arguments** that span multiple lines are vertically aligned

### Function Definitions

```clojure
;; Good: body indented 2 spaces
(defn process-message [msg]
  (validate msg)
  (handle msg))

;; Good: short function on one line
(defn add [a b] (+ a b))

;; Good: multiple arities aligned
(defn greet
  ([] (greet "World"))
  ([name] (print "Hello, " name "!")))
```

### Let Bindings

Align binding pairs vertically:

```clojure
;; Good: bindings aligned
(let [name    (get-name user)
      age     (get-age user)
      address (get-address user)]
  (format-profile name age address))

;; Also acceptable: simple bindings
(let [x 1
      y 2]
  (+ x y))
```

### Function Calls

When arguments span multiple lines, align them:

```clojure
;; Good: arguments aligned with first argument
(send-message recipient
              subject
              body
              {:priority :high})

;; Good: all arguments on subsequent lines
(send-message
  recipient
  subject
  body
  {:priority :high})
```

### Conditionals

```clojure
;; Good: branches indented
(if (valid? input)
  (process input)
  (report-error input))

;; Good: cond with aligned pairs
(cond
  (< n 0)  :negative
  (= n 0)  :zero
  (> n 0)  :positive)
```

### Threading Macros

Indent threaded forms consistently:

```clojure
;; Good: each step on its own line
(-> data
    (parse-input)
    (validate)
    (transform)
    (save!))

;; Good: for short pipelines
(-> data parse-input validate)
```

---

## Comments

### Comment Types

| Semicolons | Usage | Example |
|------------|-------|---------|
| `;;;;` | File sections/headings | `;;;; Message Handling` |
| `;;;` | Top-level explanations | `;;; This module handles...` |
| `;;` | Code block comments | `;; Validate before processing` |
| `;` | Inline/end-of-line | `(+ x y) ; sum the values` |

### Comment Rules

- Always include a space after semicolons
- Capitalize complete sentences
- Use comments to explain *why*, not *what*
- Keep comments up-to-date with code changes

```clojure
;;;; Process Management
;;; This section defines the core process lifecycle functions.
;;; Processes are lightweight execution contexts with isolated heaps.

;; Start a new process with the given function
(defn spawn [func]
  ;; Allocate a fresh heap for isolation
  (let [heap (allocate-heap default-heap-size)]
    (create-process func heap)))
```

### Annotations

Use standard annotations for marking work items:

```clojure
;; TODO: Implement proper error handling
;; FIXME: This breaks with negative numbers
;; OPTIMIZE: Consider caching this result
;; HACK: Workaround for kernel limitation
;; REVIEW: Check if this approach is correct
```

---

## Documentation

### Docstring Format

All public functions and macros must have docstrings. Docstrings appear immediately after the function name, before the parameter vector.

```clojure
;; Good: docstring placement
(defn send-message
  "Sends a message to the specified process.

  Returns true if the message was delivered successfully,
  nil if the recipient process does not exist."
  [recipient message]
  ...)
```

### Docstring Structure

1. **First line**: Complete sentence summarizing the function
2. **Blank line**: Separates summary from details
3. **Details**: Additional explanation, usage notes
4. **Parameters**: Document with backticks
5. **Examples**: Show typical usage

```clojure
(defmacro defn
  "Defines a named function.

  Creates a new function bound to `name` with the specified
  `params` and `body`. The function is added to the current
  namespace.

  Usage: (defn name [params...] body...)

  Example:
    (defn add [a b] (+ a b))
    (add 1 2) ; => 3"
  [name params & body]
  `(def ~name (fn ~name ~params ~@body)))
```

### Documentation Requirements

| Element | Required | Description |
|---------|----------|-------------|
| Summary line | Yes | First sentence describing purpose |
| Parameter docs | When non-obvious | Use backticks around param names |
| Return value | When non-obvious | What the function returns |
| Examples | Recommended | Show typical usage |
| Side effects | If any | Note any mutations or I/O |

### Macro Documentation

Macros require additional documentation elements:

```clojure
(defmacro when
  "Conditional execution with implicit do.

  Evaluates `test` and, if truthy, executes all `body` expressions
  in an implicit `do` block. Returns the value of the last expression,
  or nil if `test` is falsy.

  Usage: (when test body...)

  Example:
    (when (> x 0)
      (print \"positive\")
      x)

  Expands to:
    (if test (do body...) nil)"
  [test & body]
  `(if ~test (do ~@body) nil))
```

---

## Code Layout

### Line Length

- **Target**: 80 characters
- **Maximum**: 100 characters
- Break long forms across multiple lines

### Spacing

- Single blank line between top-level forms
- No trailing whitespace
- Files end with a single newline
- No blank lines within function bodies

```clojure
;; Good: single blank line between definitions
(defn foo [x]
  (bar x))

(defn baz [y]
  (qux y))

;; Bad: multiple blank lines
(defn foo [x]
  (bar x))


(defn baz [y]
  (qux y))
```

### Parentheses

- Never place closing parentheses on their own line
- Gather trailing parentheses together

```clojure
;; Good: trailing parens together
(defn process [data]
  (let [result (transform data)]
    (save result)))

;; Bad: closing parens on own lines
(defn process [data]
  (let [result (transform data)]
    (save result)
  )
)
```

---

## Idioms

### Prefer `when` for Single Branches

```clojure
;; Good: when for single branch
(when (valid? input)
  (process input))

;; Avoid: if with nil else
(if (valid? input)
  (process input)
  nil)
```

### Use Threading for Pipelines

```clojure
;; Good: threading macro
(-> data
    parse
    validate
    transform)

;; Avoid: nested calls
(transform (validate (parse data)))
```

### Prefer Destructuring

```clojure
;; Good: destructuring in parameters
(defn process-user [{:keys [name email age]}]
  (format-user name email age))

;; Avoid: explicit access
(defn process-user [user]
  (let [name  (get user :name)
        email (get user :email)
        age   (get user :age)]
    (format-user name email age)))
```

### Use `cond` for Multiple Conditions

```clojure
;; Good: cond for multiple conditions
(cond
  (< n 0)  (handle-negative n)
  (= n 0)  (handle-zero)
  :else    (handle-positive n))

;; Avoid: nested if
(if (< n 0)
  (handle-negative n)
  (if (= n 0)
    (handle-zero)
    (handle-positive n)))
```

---

## Error Handling

### Return Values

Prefer returning nil or meaningful error values over throwing:

```clojure
;; Good: nil for "not found"
(defn find-user [id]
  "Returns the user with the given `id`, or nil if not found."
  (get users id))

;; Good: error tuple for failures
(defn parse-config [path]
  "Parses config file. Returns [ok config] or [error reason]."
  (if (exists? path)
    [:ok (read-config path)]
    [:error :file-not-found]))
```

### Validation

Validate inputs at function boundaries:

```clojure
(defn send-message [recipient message]
  "Sends a message to the specified recipient.

  Returns nil if `recipient` is invalid."
  (when (valid-pid? recipient)
    (do-send recipient message)))
```

---

## Testing

### Test Function Naming

Test functions should clearly describe what they test:

```clojure
;; Good: descriptive test names
(deftest defn-creates-function-binding)
(deftest when-returns-nil-on-false-condition)
(deftest parser-handles-nested-expressions)

;; Avoid: vague names
(deftest test1)
(deftest defn-test)
```

### Test Structure

```clojure
(deftest when-returns-nil-on-false-condition
  ;; Arrange
  (let [condition false
        body-called (atom false)]
    ;; Act
    (when condition
      (reset! body-called true))
    ;; Assert
    (is (false? @body-called))))
```

---

## File Organization

### Standard File Structure

```clojure
;; SPDX-License-Identifier: GPL-3.0-or-later
;; Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

;; Module Name
;;
;; Module description explaining the purpose of this file
;; and its role in the system.

;;;; Dependencies (when module system exists)
;; (require '[other.module :as m])

;;;; Constants
(def max-retries 3)
(def default-timeout 5000)

;;;; Private Helpers
(defn -validate-input [input]
  ...)

;;;; Public API
(defn process-request
  "Processes an incoming request..."
  [request]
  ...)

(defn handle-response
  "Handles a response..."
  [response]
  ...)
```

---

## References

### Clojure Style Resources

- [The Clojure Style Guide](https://guide.clojure.style/) - Community standard
- [ClojureDocs Examples Style Guide](https://clojuredocs.org/examples-styleguide) - Documentation examples
- [Clojure Official Guides](https://clojure.org/guides/learn/syntax) - Language syntax

### Design Influences

- Clojure - Primary syntax and idiom inspiration
- Erlang - Process model and "let it crash" philosophy
- Scheme - Minimal, elegant Lisp design
