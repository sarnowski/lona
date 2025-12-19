# Type Predicates
> **Status**: Type predicates are not yet implemented as callable functions. The underlying type infrastructure exists in `lona-core`, but the native function bindings have not been added yet.

Type predicates inspect runtime type tags and return boolean values.

## Available Predicates

| Function | Description | Status |
|----------|-------------|--------|
| `nil?` | Is the value nil? | *(Planned)* |
| `boolean?` | Is the value a boolean? | *(Planned)* |
| `integer?` | Is the value an integer? | *(Planned)* |
| `float?` | Is the value a float? | *(Planned)* |
| `ratio?` | Is the value a ratio? | *(Planned)* |
| `string?` | Is the value a string? | *(Planned)* |
| `symbol?` | Is the value a symbol? | *(Planned)* |
| `keyword?` | Is the value a keyword? | *(Planned)* — requires Keyword type |
| `binary?` | Is the value a binary buffer? | *(Planned)* — requires Binary type |
| `list?` | Is the value a list? | *(Planned)* |
| `vector?` | Is the value a vector? | *(Planned)* |
| `map?` | Is the value a map? | *(Planned)* |
| `set?` | Is the value a set? | *(Planned)* — requires Set type |
| `fn?` | Is the value a function? | *(Planned)* |
| `coll?` | Is the value a collection (list, vector, map, or set)? | *(Planned)* |
| `seq?` | Is the value a sequence? | *(Planned)* |

## Examples

```clojure
(nil? nil)        ; => true
(list? '(1 2 3))  ; => true
(vector? [1 2])   ; => true
(fn? +)           ; => true
```

---

