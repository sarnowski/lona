# Specification Tests

This document describes how to write testable examples in Lonala specification documents. Examples marked with assertions are automatically extracted and verified against the VM implementation.

---

## Quick Reference

```clojure
;; Setup line (no assertion, establishes state)
(def x 42)

;; Assertion: expression followed by the arrow marker
(+ x 8)  ; => 50

;; Error assertion
(/ 1 0)  ; => ERROR

;; Specific error type
(nth [] 5)  ; => ERROR :index-out-of-bounds

;; Mark unimplemented features
(lazy-seq [1 2 3])  ; => (1 2 3)  @todo
```

---

## Test Blocks

A markdown code block becomes a **test block** when it contains at least one `; =>` assertion marker. Code blocks without assertions are documentation-only and ignored by the test extractor.

**Test block** (will be executed):

````markdown
```clojure
(+ 1 2)  ; => 3
```
````

**Documentation block** (ignored):

````markdown
```clojure
;; This pattern shows how to structure a handler
(defn my-handler [msg]
  (match msg
    [:request id data] (handle-request id data)
    [:shutdown] (exit :normal)))
```
````

---

## Assertions

An assertion tests that an expression evaluates to an expected value.

### Syntax

```text
EXPRESSION  ; => EXPECTED  [@TAG...]
```

| Component | Description |
|-----------|-------------|
| `EXPRESSION` | Any Lonala expression |
| `; =>` | Assertion marker (semicolon, space, arrow) |
| `EXPECTED` | Expected result value, or `ERROR` |
| `@TAG` | Optional tags (see [Tags](#tags)) |

### Value Assertions

The expression must evaluate to exactly the expected value:

```clojure
(+ 1 2)        ; => 3
(str "a" "b")  ; => "ab"
(first [1 2])  ; => 1
(empty? {})    ; => true
```

### Error Assertions

Use `ERROR` to assert that evaluation causes an error:

```clojure
(/ 1 0)      ; => ERROR
(nth [] 10)  ; => ERROR
```

Optionally specify the error type:

```clojure
(nth [] 10)     ; => ERROR :out-of-bounds  @todo
(+ 1 :foo)      ; => ERROR :type-error
(undefined-fn)  ; => ERROR :undefined
```

---

## Setup Lines

Lines without `; =>` are **setup lines**. They execute for side effects and establish state for subsequent assertions. All lines in a block share the same evaluation context.

```clojure
;; @todo
(def buf (bytebuf-alloc 8))      ; setup
(bytebuf-write8! buf 0 42)       ; setup
(bytebuf-read8 buf 0)            ; => 42
(bytebuf-read8 buf 1)            ; => 0
```

If a setup line causes an error, the entire block fails.

---

## Multi-line Expressions

Expressions can span multiple lines. Place the `; =>` marker after the closing delimiter:

```clojure
(match [:ok 42]
  [:ok result] result
  [:error _] nil)  ; => 42
```

```clojure
;; @todo
(map inc
     {1 2 3})  ; => {2 3 4}
```

---

## Tags

Tags modify how assertions are processed. They appear after the expected value, prefixed with `@`.

### Available Tags

| Tag | Meaning |
|-----|---------|
| `@todo` | Feature not yet implemented. Test expected to fail. |
| `@x86_64` | Only run on x86_64 architecture |
| `@aarch64` | Only run on aarch64 architecture |

### Line-Level Tags

Apply to a single assertion:

```clojure
;; @todo
(bytebuf-read8 buf 0)   ; => 42
(bytebuf-read16 buf 0)  ; => 10794
```

### Block-Level Tags

A comment starting with `;; @` at the beginning of a block applies to ALL assertions:

```clojure
;; @todo
(lazy-seq [1 2 3])  ; => (1 2 3)
(lazy-take 2 xs)    ; => (1 2)
(lazy-drop 1 xs)    ; => (2 3)
```

Architecture-specific blocks:

```clojure
;; @x86_64 @todo
(port-in8 0x3F8)      ; => 0
(port-out8 0x3F8 65)  ; => nil
```

Line tags combine with block tags (they don't replace them).

**Note:** Block-level tags only work at the **first line** of a code block. A `;; @todo` comment in the middle of a block is treated as a regular comment and does not apply to subsequent tests. To mark individual tests within a block, use line-level tags.

---

## Test Results

| Condition | Result |
|-----------|--------|
| Assertion passes | `pass` |
| Assertion fails | `fail` (build fails) |
| `@todo` assertion fails | `todo` (expected, build passes) |
| `@todo` assertion passes | `todo_fail` (unexpected - remove `@todo`!) |

---

## Best Practices

### One Concept Per Block

Test one function or concept per block. This makes failures easier to diagnose:

```clojure
;; Good: focused on `first`
(first [1 2 3])  ; => 1
(first [])       ; => nil
(first nil)      ; => nil
```

### Include Edge Cases

Always test boundaries and edge cases:

```clojure
;; @todo
(nth {1 2 3} 0)   ; => 1
(nth {1 2 3} 2)   ; => 3
(nth {1 2 3} -1)  ; => nil
(nth {1 2 3} 10)  ; => nil
(nth {} 0)        ; => nil
```

### Show Error Cases

Document when and how functions fail:

```clojure
(/ 10 2)  ; => 5
(/ 10 0)  ; => ERROR :division-by-zero
```

### Use Descriptive Setup

When setup is needed, keep it minimal and clear:

```clojure
(def m %{:a 1 :b 2})
(get m :a)  ; => 1
(get m :c)  ; => nil
```

### Mark Unimplemented Features

Use `@todo` for features that are specified but not yet implemented:

```clojure
;; @todo
(reduce + 0 {1 2 3})  ; => 6
```

This documents intent while allowing the build to pass.
