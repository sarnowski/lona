# lona.time

Time intrinsics.

---

## Monotonic Time

### `monotonic-time`

Get monotonic timestamp.

```clojure
(monotonic-time)  ; → nanoseconds since boot
```

```clojure
;; @todo
(integer? (monotonic-time))  ; => true
(>= (monotonic-time) 0)      ; => true
```

Monotonically increasing:

```clojure
;; @todo
(def t1 (monotonic-time))
(def t2 (monotonic-time))
(>= t2 t1)  ; => true
```

Monotonically increasing, unaffected by clock adjustments. Use for measuring elapsed time and timeouts.

---

## Sleep

### `sleep`

Pause current process.

```clojure
(sleep ms)  ; → :ok
```

```clojure
;; @todo
(sleep 0)   ; => :ok
(sleep 1)   ; => :ok
```

Sleep error cases:

```clojure
;; @todo
(sleep -1)   ; => ERROR :badarg
(sleep nil)  ; => ERROR :badarg
```

Sleep takes non-zero time:

```clojure
;; @todo
(def t1 (monotonic-time))
(sleep 10)
(def t2 (monotonic-time))
(> t2 t1)  ; => true
```

Yields CPU for at least `ms` milliseconds. Other processes continue running.

---

## System Time

### `system-time`

Get wall clock time.

```clojure
(system-time)  ; → nanoseconds since epoch
```

```clojure
;; @todo
(integer? (system-time))  ; => true
(> (system-time) 0)       ; => true

;; System time should be positive and reasonable
(> (system-time) 1000000000000000000)  ; => true  ; After year 2001 in nanoseconds
```

May jump due to clock adjustments. Use `monotonic-time` for durations.

```clojure
;; @todo
;; NOTE: Unlike monotonic-time, system-time can go backwards
;; due to NTP adjustments or manual clock changes.
;; For measuring durations, always use monotonic-time.
(def t1 (system-time))
(def t2 (system-time))
;; t2 >= t1 is NOT guaranteed for system-time
(integer? t1)  ; => true
(integer? t2)  ; => true
```

---

## Appendix: Expected Derived Functions

The following are **not intrinsics** and should be implemented in Lonala:

- `timeout` — create timeout reference
- `with-timeout` — execute with deadline
- `after` — schedule delayed message to self
