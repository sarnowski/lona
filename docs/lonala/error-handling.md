# Error Handling

This document specifies Lonala's error handling philosophy and mechanisms, explaining when to use each approach.

## Overview

Lonala provides two complementary error handling mechanisms:

| Mechanism | Purpose | Stack Behavior | When to Use |
|-----------|---------|----------------|-------------|
| **Result Tuples** | Expected failures | Returns to caller | Domain logic failures |
| **Conditions** | Unexpected failures | Preserves stack | System failures, bugs |

These mechanisms are **not in conflict** — they serve different purposes and work together.

## Result Tuples

Result tuples handle **expected** failures that are part of normal domain logic.

### Syntax

```clojure
;; Success
{:ok value}

;; Failure
{:error reason}

;; Reason can be a keyword
{:error :not-found}

;; Or a rich map
{:error {:type :validation
         :field :email
         :message "Invalid email format"}}
```

### When to Use

Use result tuples when:
- The failure is **expected** in normal operation
- The caller **should** handle this case
- The failure is part of the **domain logic**

Examples:
- User not found in database
- Validation failed
- Permission denied
- Resource already exists
- Input parsing failed

### Pattern

```clojure
;; Function returns result tuple
(defn find-user [id]
  (if-let [user (db/get-user id)]
    {:ok user}
    {:error :not-found}))

;; Caller handles both cases
(case (find-user 123)
  {:ok user} (process-user user)
  {:error :not-found} (create-user 123)
  {:error reason} (log-error reason))
```

### Composing Results

```clojure
;; Chain fallible operations with ok->
(ok-> (find-user id)
      (validate-user)
      (update-last-login)
      (generate-token))

;; Each step returns {:ok _} or {:error _}
;; Pipeline short-circuits on first error

;; Handle errors at the end
(case result
  {:ok token} (respond-success token)
  {:error reason} (respond-error reason))
```

### Standard Functions

```clojure
(ok? {:ok 42})           ;; => true
(error? {:error :fail})  ;; => true

(unwrap! {:ok 42})       ;; => 42
(unwrap! {:error :fail}) ;; => panics!

(unwrap-or {:ok 42} 0)   ;; => 42
(unwrap-or {:error _} 0) ;; => 0

(map-ok {:ok 5} inc)     ;; => {:ok 6}
(map-ok {:error e} inc)  ;; => {:error e}
```

## Conditions and Restarts

Conditions handle **unexpected** failures that represent bugs, system errors, or exceptional circumstances.

### Key Properties

Unlike exceptions in most languages, conditions:
1. **Do not unwind the stack** immediately
2. **Preserve full context** (locals, call stack)
3. **Offer recovery options** (restarts)
4. Allow the **debugger to intervene** (in debug mode)

### Syntax

#### Signaling Conditions

```clojure
;; Signal a condition
(signal :file-not-found {:path path})

;; With panic! (convenience for unrecoverable errors)
(panic! "Assertion failed" {:expected x :actual y})
```

#### Defining Restarts

```clojure
(restart-case
  ;; Protected expression
  (read-file path)

  ;; Restarts offered if error occurs
  (:retry []
    "Try reading the file again"
    (read-file path))

  (:use-default []
    "Use default content"
    default-content)

  (:use-value [value]
    "Provide content manually"
    value))
```

#### Handling Conditions

```clojure
;; High-level handler decides what to do
(handler-bind
  [:file-not-found
   (fn [condition]
     (if (= (:path condition) "/etc/config")
       (invoke-restart :use-default)
       (invoke-restart :retry)))]

  (start-application))
```

### When to Use

Use conditions when:
- The failure is **unexpected** or represents a bug
- The caller **should not** have to handle it explicitly
- You want **debugger intervention** possible
- You need **stack context preserved** for diagnosis

Examples:
- File I/O errors (file should exist but doesn't)
- Network timeouts (transient infrastructure issues)
- Assertion failures (bugs)
- Resource exhaustion (out of memory)
- Hardware errors

### Behavior by Mode

| Condition | Production Mode | Debug Mode |
|-----------|-----------------|------------|
| Handled by `handler-bind` | Handler runs | Handler runs |
| Unhandled | Process crashes | Debugger activates |
| `panic!` | Process crashes | Debugger activates |

## Decision Tree

```
┌─────────────────────────────────────────────────────────────────┐
│         Is this failure EXPECTED in normal operation?           │
└───────────────────────────┬─────────────────────────────────────┘
                            │
              ┌─────────────┴─────────────┐
              │                           │
             YES                          NO
              │                           │
              ▼                           ▼
    ┌─────────────────┐         ┌─────────────────┐
    │  Result Tuples  │         │   Conditions    │
    │  {:ok _}        │         │   (signal ...)  │
    │  {:error _}     │         │   (panic! ...)  │
    └─────────────────┘         └─────────────────┘
              │                           │
              ▼                           ▼
    ┌─────────────────┐         ┌─────────────────┐
    │ Caller handles  │         │ Production:     │
    │ explicitly      │         │   crash/restart │
    │                 │         │ Debug:          │
    │                 │         │   pause/inspect │
    └─────────────────┘         └─────────────────┘
```

### Concrete Examples

| Situation | Mechanism | Rationale |
|-----------|-----------|-----------|
| User not found | Result tuple | Expected, caller should handle |
| Validation failed | Result tuple | Expected, part of domain logic |
| Config file missing | Condition | System should have config |
| Division by zero | Condition | Bug in code |
| Network timeout | Condition | Transient, retry or fail |
| Out of memory | Condition | System failure |
| Permission denied | Result tuple | Expected access control |
| Invariant violated | Condition | Bug, should never happen |

## Combining Both Mechanisms

Result tuples and conditions work together:

```clojure
(defn process-order [order-id]
  ;; Result tuple for expected "not found"
  (case (find-order order-id)
    {:ok order}
      ;; Condition for unexpected system errors
      (restart-case
        (charge-payment order)

        (:retry []
          "Retry payment"
          (charge-payment order))

        (:mark-pending []
          "Mark order as pending"
          (set-order-status! order :payment-pending)
          {:ok :pending}))

    {:error :not-found}
      {:error :order-not-found}))
```

## The `panic!` Special Form

`panic!` is a convenience for signaling unrecoverable conditions:

```clojure
;; These are equivalent
(panic! "Message" data)
(signal :panic {:message "Message" :data data})
```

### Behavior

- **Production mode**: Terminates the process immediately
- **Debug mode**: Pauses and presents debugger UI

### When to Use

```clojure
;; Assertions (bug detection)
(when (neg? balance)
  (panic! "Balance cannot be negative" {:balance balance}))

;; Unreachable code
(case status
  :active (handle-active)
  :inactive (handle-inactive)
  (panic! "Unknown status" {:status status}))

;; Invariant violations
(when (not= (count items) expected-count)
  (panic! "Item count mismatch" {:expected expected-count
                                  :actual (count items)}))
```

### NOT for Expected Errors

Do **not** use `panic!` for expected failures:

```clojure
;; WRONG: This is an expected case
(defn find-user [id]
  (if-let [user (db/get id)]
    user
    (panic! "User not found")))  ; NO!

;; RIGHT: Return result tuple
(defn find-user [id]
  (if-let [user (db/get id)]
    {:ok user}
    {:error :not-found}))
```

## Standard Library Conventions

### Functions That May Fail

Functions that may fail due to **expected** reasons return result tuples:

```clojure
;; Returns {:ok user} or {:error :not-found}
(user/find-by-id id)

;; Returns {:ok parsed} or {:error {:type :parse-error ...}}
(json/parse string)

;; Returns {:ok file-content} or {:error :not-found}
(file/read-if-exists path)
```

### Functions That Should Not Fail

Functions that should not fail (given valid input) use conditions:

```clojure
;; Signals condition if file doesn't exist
(file/read! path)

;; Signals condition on parse error (use parse for result tuple)
(json/parse! string)

;; Signals condition if index out of bounds
(vector/get! vec index)
```

The `!` suffix convention indicates a function that may signal conditions.

### Unwrap Functions

`unwrap!` converts a result tuple to a condition:

```clojure
;; If {:ok value}, returns value
;; If {:error reason}, signals condition
(defn process [data]
  (let [user (unwrap! (find-user id))]  ; May panic
    (do-something-with user)))
```

## Restarts Reference

### Built-in Restarts

Every condition has these restarts available:

| Restart | Description |
|---------|-------------|
| `:abort` | Crash the process, trigger supervisor |
| `:continue` | Continue execution (if possible) |
| `:use-value` | Return a provided value |
| `:retry` | Retry the operation |

### Defining Custom Restarts

```clojure
(restart-case
  (fetch-data url)

  ;; No arguments
  (:use-cached []
    "Use cached data instead"
    (get-cache url))

  ;; With arguments (provided by handler or user)
  (:use-fallback [fallback-url]
    "Use a different URL"
    (fetch-data fallback-url))

  ;; With interactive prompt (for debugger)
  (:use-value [value]
    :interactive (fn [] [(prompt "Enter value: ")])
    "Provide a value manually"
    value))
```

### Invoking Restarts

```clojure
;; From handler-bind
(handler-bind
  [:network-error
   (fn [c]
     (invoke-restart :use-cached))]
  (fetch-all-data))

;; With arguments
(handler-bind
  [:network-error
   (fn [c]
     (invoke-restart :use-fallback "http://backup.example.com"))]
  (fetch-data url))

;; From debugger UI
proc-debug[0]> 2  ; select restart by number
```

## Best Practices

### 1. Be Explicit About Failure

```clojure
;; GOOD: Clear that this may fail
(case (find-user id)
  {:ok user} (process user)
  {:error _} (handle-missing))

;; BAD: Hides failure, will panic unexpectedly
(let [user (unwrap! (find-user id))]
  (process user))
```

### 2. Fail Fast in Development

```clojure
;; Use assertions liberally
(defn transfer [from to amount]
  (assert (pos? amount) "Amount must be positive")
  (assert (not= from to) "Cannot transfer to self")
  ...)
```

### 3. Provide Rich Error Context

```clojure
;; GOOD: Rich context for debugging
{:error {:type :validation
         :field :email
         :value "not-an-email"
         :message "Invalid email format"}}

;; BAD: No context
{:error :validation-failed}
```

### 4. Document Error Cases

```clojure
(defn create-user
  "Creates a new user.

  Returns:
    {:ok user} - User created successfully
    {:error :email-taken} - Email already registered
    {:error :invalid-email} - Email format invalid
    {:error :weak-password} - Password doesn't meet requirements"
  [email password]
  ...)
```

### 5. Use the Right Mechanism

| If you're thinking... | Use |
|-----------------------|-----|
| "The caller should handle this" | Result tuple |
| "This should never happen" | Condition/panic |
| "Let the supervisor handle it" | Condition/panic |
| "I need to inspect this" | Condition (with debugger) |
