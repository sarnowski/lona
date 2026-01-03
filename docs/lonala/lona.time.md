# lona.time

Time intrinsics.

---

## Monotonic Time

### `monotonic-time`

Get monotonic timestamp.

```clojure
(monotonic-time)  ; → nanoseconds since boot
```

Monotonically increasing, unaffected by clock adjustments. Use for measuring elapsed time and timeouts.

---

## Sleep

### `sleep`

Pause current process.

```clojure
(sleep ms)  ; → :ok
```

Yields CPU for at least `ms` milliseconds. Other processes continue running.

---

## System Time

### `system-time`

Get wall clock time.

```clojure
(system-time)  ; → nanoseconds since epoch
```

May jump due to clock adjustments. Use `monotonic-time` for durations.

---

## Appendix: Expected Derived Functions

The following are **not intrinsics** and should be implemented in Lonala:

- `timeout` — create timeout reference
- `with-timeout` — execute with deadline
- `after` — schedule delayed message to self
