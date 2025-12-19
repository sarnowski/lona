# Regular Expressions
> **Status**: *(Planned)*

Regular expression operations for pattern matching in strings.

## Functions

| Function | Syntax | Description |
|----------|--------|-------------|
| `re-pattern` | `(re-pattern s)` | Compile string to regex pattern |
| `re-find` | `(re-find re s)` | Find first match in string |
| `re-matches` | `(re-matches re s)` | Match entire string against pattern |
| `re-seq` | `(re-seq re s)` | Return lazy seq of all matches |
| `re-groups` | `(re-groups m)` | Return groups from most recent match |

## Examples

### Creating Patterns

```clojure
;; Create pattern from string
(def digit-pattern (re-pattern "\\d+"))

;; Or use regex literal
(def digit-pattern #"\d+")
```

### Finding Matches

```clojure
;; Find first match
(re-find #"\d+" "abc123def")
; => "123"

;; Find with groups
(re-find #"(\d+)-(\d+)" "phone: 555-1234")
; => ["555-1234" "555" "1234"]
```

### Matching Entire String

```clojure
(re-matches #"\d+" "123")     ; => "123"
(re-matches #"\d+" "abc123")  ; => nil (doesn't match entire string)
```

### Finding All Matches

```clojure
(re-seq #"\d+" "a1b2c3")
; => ("1" "2" "3")
```

### Pattern Flags

```clojure
;; Case-insensitive
(re-find #"(?i)hello" "HELLO world")
; => "HELLO"
```

## Regex Literal Syntax

**Syntax**: `#"pattern"`

The `#"..."` reader macro creates a compiled regular expression pattern. This is syntactic sugar for `(re-pattern "...")` but more convenient and allows the pattern to be compiled at read time.

```clojure
#"\d+"              ; matches one or more digits
#"[a-zA-Z]+"        ; matches one or more letters
#"hello\s+world"    ; matches "hello" followed by whitespace and "world"
```

---

