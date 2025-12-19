# Binary Operations
> **Status**: *(Planned)* — Requires the Binary type to be implemented first.

Operations on raw byte buffers for systems programming.

## Functions

| Function | Syntax | Description |
|----------|--------|-------------|
| `make-binary` | `(make-binary size)` | Allocate zeroed byte buffer |
| `binary-len` | `(binary-len buf)` | Get buffer length |
| `binary-get` | `(binary-get buf index)` | Get byte at index (0-255) |
| `binary-set` | `(binary-set buf index byte)` | Set byte at index |
| `binary-slice` | `(binary-slice buf start end)` | Zero-copy view |
| `binary-copy!` | `(binary-copy! dst dst-off src src-off len)` | Copy bytes |

## Examples

```clojure
(def buf (make-binary 4))
(binary-set buf 0 0xFF)
(binary-get buf 0)        ; => 255
(binary-len buf)          ; => 4
```

## Use Cases

- Network packet parsing and construction
- Device driver buffers
- Binary file I/O
- Memory-mapped I/O data

---

