# Appendix F: Error Handling Idioms
This appendix provides practical patterns for error handling in Lonala.

## F.1 Philosophy

Lonala follows the Erlang/BEAM philosophy:

1. **Expected failures are values, not exceptions**
   - Network timeout? Return `{:error :timeout}`
   - File not found? Return `{:error :not-found}`
   - Invalid input? Return `{:error {:validation ...}}`

2. **Unexpected failures crash the process**
   - Bug in code? `panic!`
   - Invariant violation? `panic!`
   - Corrupted data? `panic!`

3. **Supervisors handle crashes**
   - Crashed processes are restarted automatically
   - System stays up even when components fail
   - "Let it crash" instead of defensive programming

## F.2 Function Naming Convention

| Pattern | Meaning | Example |
|---------|---------|---------|
| `foo` | Returns `{:ok value}` or `{:error reason}` | `(read-file path)` |
| `foo!` | Returns value or panics | `(read-file! path)` |
| `foo?` | Returns boolean | `(file-exists? path)` |

```clojure
;; Define both variants
(defn fetch-user [id]
  (case (db-lookup :users id)
    nil  {:error :not-found}
    user {:ok user}))

(defn fetch-user! [id]
  (unwrap! (fetch-user id)))
```

## F.3 Error Propagation Patterns

### Pattern 1: Direct Case Matching

```clojure
(defn process-order [order-id]
  (case (fetch-order order-id)
    {:ok order}
      (case (validate-order order)
        {:ok valid-order}
          (case (charge-payment valid-order)
            {:ok receipt}    {:ok {:order valid-order :receipt receipt}}
            {:error reason}  {:error {:step :payment :cause reason}})
        {:error reason}      {:error {:step :validation :cause reason}})
    {:error reason}          {:error {:step :fetch :cause reason}}))
```

### Pattern 2: Using `with` (Preferred)

```clojure
(defn process-order [order-id]
  (with [{:ok order}       (fetch-order order-id)
         {:ok valid-order} (validate-order order)
         {:ok receipt}     (charge-payment valid-order)]
    {:ok {:order valid-order :receipt receipt}}
    (else {:step step :cause reason}
      {:error {:step step :cause reason}})))
```

### Pattern 3: Using `ok->`

```clojure
(defn process-order [order-id]
  (ok-> order-id
        fetch-order
        validate-order
        charge-payment))
```

## F.4 Systems Programming Patterns

### Driver: Expected vs Fatal Errors

```clojure
(defn uart-driver [base-addr]
  ;; Initialization failure is fatal
  (when (not (device-present? base-addr))
    (panic! "UART not present" {:addr base-addr}))

  (loop []
    (receive
      ;; TX request - may fail (expected)
      {:tx byte from}
        (case (uart-write base-addr byte)
          {:ok _}           (send from {:ok :sent})
          {:error :tx-full} (send from {:error :busy}))

      ;; RX with timeout - expected condition
      {:rx from timeout}
        (case (uart-read base-addr timeout)
          {:ok byte}        (send from {:ok byte})
          {:error :timeout} (send from {:error :timeout})
          {:error :overrun} (do
                              (inc-counter :rx-overrun)
                              (send from {:error :overrun}))))
    (recur)))
```

### Network Stack: Resource Management

```clojure
(defn tcp-connect [addr port]
  (with [{:ok sock}    (allocate-socket)
         {:ok _}       (send-syn sock addr port)
         {:ok _}       (wait-syn-ack sock 30000)]
    {:ok sock}
    (else err
      ;; Clean up on any failure
      (when sock (release-socket sock))
      {:error err})))
```

## F.5 Anti-Patterns to Avoid

### Ignoring Errors

```clojure
;; BAD: Error silently ignored
(defn bad-example []
  (let [result (might-fail)]
    (when (ok? result)
      (process (unwrap! result)))))
```

### Handle or Propagate

```clojure
;; GOOD: Explicitly handle or propagate
(defn good-example []
  (case (might-fail)
    {:ok value}    (process value)
    {:error _}     {:error :processing-failed}))
```

### Panic for Expected Failures

```clojure
;; BAD: Panicking on expected condition
(defn bad-read-config [path]
  (when (not (file-exists? path))
    (panic! "Config not found"))  ; Don't do this!
  ...)
```

### Return Error for Expected Failures

```clojure
;; GOOD: Return error tuple
(defn good-read-config [path]
  (if (file-exists? path)
    {:ok (parse-config (read-file! path))}
    {:error :config-not-found}))
```

## F.6 Standard Error Reasons

Common error reasons used throughout the standard library:

| Reason | Meaning |
|--------|---------|
| `:not-found` | Requested item doesn't exist |
| `:timeout` | Operation timed out |
| `:busy` | Resource temporarily unavailable |
| `:permission-denied` | Capability/permission missing |
| `:invalid-argument` | Argument failed validation |
| `:resource-exhausted` | Out of memory, handles, etc. |
| `:not-supported` | Operation not supported |
| `:already-exists` | Item already exists |
| `:io-error` | Low-level I/O failure |
| `:closed` | Resource was closed |

## F.7 Comparison with Other Languages

| Lonala | Rust | Elixir | Clojure |
|--------|------|--------|---------|
| `{:ok value}` | `Ok(value)` | `{:ok, value}` | (varies) |
| `{:error reason}` | `Err(e)` | `{:error, reason}` | (varies) |
| `panic!` | `panic!` | `raise` / exit | `throw` |
| `with` | `?` operator | `with` | (varies) |
| `unwrap!` | `.unwrap()` | `elem(..., 1)` | (varies) |

---

