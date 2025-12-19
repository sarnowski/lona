# Process and seL4 Primitives
> **Status**: *(Planned)* — See [Concurrency](../concurrency.md) for full process documentation.

## Process Primitives

| Function | Syntax | Description |
|----------|--------|-------------|
| `spawn` | `(spawn fn)` | Create new process |
| `self` | `(self)` | Get current process ID |
| `exit` | `(exit reason)` | Exit current process |
| `send` | `(send pid msg)` | Send message to process |

## Examples

```clojure
;; Spawn a new process
(spawn (fn [] (print "Hello from process!")))

;; Get current process ID
(self)

;; Exit current process
(exit :normal)

;; Send message to process
(send pid {:type :greeting :text "Hello"})
```

## seL4 / Domain Primitives

Low-level seL4 operations for domain isolation.

| Function | Syntax | Description |
|----------|--------|-------------|
| `domain-create` | `(domain-create opts)` | Create isolated domain |
| `cap-grant` | `(cap-grant domain cap)` | Grant capability to domain |
| `cap-revoke` | `(cap-revoke domain cap)` | Revoke capability |

## See Also

- [Concurrency](../concurrency.md) — Full process, message passing, and supervision documentation

---

