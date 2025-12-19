# Time
> **Status**: *(Planned)*

Time-related primitives.

## Functions

| Function | Syntax | Description |
|----------|--------|-------------|
| `now-ms` | `(now-ms)` | Current time in milliseconds |
| `send-after` | `(send-after pid delay msg)` | Send message after delay |

## Examples

```clojure
(now-ms)                  ; => 1234567890
(send-after (self) 1000 :timeout)  ; Send :timeout to self after 1 second
```

## Use Cases

### Timeout Handling

```clojure
(defn with-timeout [timeout-ms operation]
  (send-after (self) timeout-ms :timeout)
  (receive
    {:result value} {:ok value}
    :timeout        {:error :timeout}))
```

### Periodic Tasks

```clojure
(defn ticker [interval-ms]
  (loop []
    (send-after (self) interval-ms :tick)
    (receive
      :tick (do
              (perform-periodic-task)
              (recur)))))
```

---

