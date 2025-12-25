# Symbol Operations
> **Status**: Implemented. Both `symbol` and `gensym` are fully functional.

## Functions

| Function | Syntax | Description |
|----------|--------|-------------|
| `symbol` | `(symbol name)` | Create/intern a symbol |
| `gensym` | `(gensym)` or `(gensym prefix)` | Generate unique symbol |

## Examples

```clojure
(symbol "foo")    ; => foo
(gensym)          ; => G__123
(gensym "temp")   ; => temp__124
```

## Use in Macros

`gensym` is critical for writing hygienic macros that avoid variable capture:

```clojure
;; Without gensym - vulnerable to variable capture
(defmacro bad-twice [expr]
  `(let [x ~expr]
     (+ x x)))

;; With gensym - hygienic
(defmacro good-twice [expr]
  (let [x (gensym "x")]
    `(let [~x ~expr]
       (+ ~x ~x))))
```

---

