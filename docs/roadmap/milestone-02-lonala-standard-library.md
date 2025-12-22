## Milestone 2: Lonala Standard Library

**Goal**: Implement complete standard library in Lonala with test coverage.

**Prerequisite**: Milestone 1 complete

> **Bootstrap Note**: Phase 2.1 (Test Framework) uses `native-print` (Task 1.8.23) until Milestone 3 provides the Lonala UART driver.

### Phase 2.1: Test Framework

#### Task 2.1.1: Test Namespace Foundation

**Description**: Create `lona.test` namespace with core testing primitives.

**Files to create**:
- `lona/test.lona`

**Requirements**:
- `deftest` macro for defining tests
- `is` macro for assertions
- `are` macro for multiple assertions
- `testing` macro for grouping
- Test result tracking

**Tests**: Bootstrap tests for the test framework itself

**Estimated effort**: 1-2 context windows

---

#### Task 2.1.2: Test Runner

**Description**: Implement test discovery and execution.

**Files to modify**:
- `lona/test.lona`

**Requirements**:
- `run-tests` function
- `run-all-tests` for namespace
- Test filtering by name/tag
- Result reporting (pass/fail/error counts)
- Failure details with expected vs actual

**Estimated effort**: 1-2 context windows

---

#### Task 2.1.3: Fixtures and Setup/Teardown

**Description**: Add test lifecycle support.

**Files to modify**:
- `lona/test.lona`

**Requirements**:
- `use-fixtures` for setup/teardown
- `:each` fixtures (per test)
- `:once` fixtures (per namespace)
- Fixture composition

**Estimated effort**: 1 context window

---

#### Task 2.1.4: Test Integration with Build

**Description**: Integrate Lonala tests with `make test`.

**Files to modify**:
- `Makefile`
- `crates/lona-runtime/src/main.rs`

**Requirements**:
- `make test` runs Rust tests then Lonala tests
- Test runner loads `test/**/*.lona`
- Exit code reflects test results
- Summary output

**Estimated effort**: 1 context window

---

### Phase 2.2: Core Functions

#### Task 2.2.1: Sequence Functions - Basic

**Description**: Implement fundamental sequence operations.

**Files to create**:
- `lona/core/seq.lona`

**Requirements**:
- `seq` - native primitive (Task 1.8.26), add convenience wrapper if needed
- `first`, `rest` - native primitives, document in this namespace
- `next` - `(defn next [coll] (seq (rest coll)))`
- `ffirst`, `fnext`, `nnext` - convenience compositions
- `second`, `last` - position access

**Note**: `seq`, `first`, `rest` are native primitives (Milestone 1). This task provides wrappers and convenience functions built on them.

**Tests**: Full coverage for each function

**Estimated effort**: 1 context window

---

#### Task 2.2.2: Sequence Functions - Transformation

**Description**: Implement map, filter, reduce.

**Files to modify**:
- `lona/core/seq.lona`

**Requirements**:
- `map` - transform elements
- `filter` - select elements
- `remove` - reject elements
- `reduce` - fold to single value
- `reduce-kv` - reduce with keys

**Tests**: Full coverage including edge cases

**Estimated effort**: 1-2 context windows

---

#### Task 2.2.3: Sequence Functions - Construction

**Description**: Implement sequence builders.

**Files to modify**:
- `lona/core/seq.lona`

**Requirements**:
- `range` - numeric range
- `repeat` - repeated value
- `repeatedly` - repeated function calls
- `iterate` - iterative generation
- `cycle` - infinite cycling

**Tests**: Full coverage

**Estimated effort**: 1 context window

---

#### Task 2.2.4: Sequence Functions - Combination

**Description**: Implement sequence combinators.

**Files to modify**:
- `lona/core/seq.lona`

**Requirements**:
- `concat` - join sequences (native wrapper)
- `mapcat` - map then concat
- `interleave` - alternate elements
- `interpose` - insert separator
- `zip` - pair elements

**Tests**: Full coverage

**Estimated effort**: 1 context window

---

#### Task 2.2.5: Sequence Functions - Partitioning

**Description**: Implement sequence splitting.

**Files to modify**:
- `lona/core/seq.lona`

**Requirements**:
- `take`, `drop` - positional
- `take-while`, `drop-while` - predicate
- `partition`, `partition-by` - grouping
- `split-at`, `split-with` - splitting

**Tests**: Full coverage

**Estimated effort**: 1 context window

---

#### Task 2.2.6: Higher-Order Functions

**Description**: Implement function combinators.

**Files to create**:
- `lona/core/fn.lona`

**Requirements**:
- `identity` - return argument
- `constantly` - return constant
- `comp` - compose functions
- `partial` - partial application
- `complement` - negate predicate
- `juxt` - apply multiple fns

**Tests**: Full coverage

**Estimated effort**: 1 context window

---

#### Task 2.2.7: apply Wrapper

**Description**: Lonala wrapper for native `apply` primitive.

**Files to modify**:
- `lona/core/fn.lona`

**Requirements**:
- Wraps native `apply` (Task 1.8.20) which handles basic `(apply f args)`
- Adds multi-argument convenience: `(apply f x y args)` prepends x, y to args
- Variadic form: `(apply f x y z & more)` for arbitrary leading args

**Implementation**:
```clojure
(defn apply
  ([f args] (native-apply f args))
  ([f x args] (native-apply f (cons x args)))
  ([f x y args] (native-apply f (cons x (cons y args))))
  ([f x y z & args] (native-apply f (list* x y z args))))
```

**Tests**: Various application patterns including multi-arg forms

**Estimated effort**: 0.5 context windows

---

### Phase 2.3: Control Flow Macros

#### Task 2.3.1: Conditional Macros

**Description**: Implement conditional control flow.

**Files to create**:
- `lona/core/control.lona`

**Requirements**:
- `when-not` - opposite of when
- `if-not` - opposite of if
- `cond` - multi-way conditional
- `condp` - predicate conditional
- `case` - constant dispatch (macro layer)

**Tests**: Full coverage

**Estimated effort**: 1-2 context windows

---

#### Task 2.3.2: Let Variants

**Description**: Implement let variations.

**Files to modify**:
- `lona/core/control.lona`

**Requirements**:
- `if-let` - conditional binding
- `when-let` - conditional binding with when
- `if-some` - nil-specific conditional
- `when-some` - nil-specific when

**Tests**: Full coverage

**Estimated effort**: 1 context window

---

#### Task 2.3.3: Boolean Macros

**Description**: Implement short-circuit booleans.

**Files to modify**:
- `lona/core/control.lona`

**Requirements**:
- `and` - short-circuit and
- `or` - short-circuit or
- `not` - already implemented, ensure correct

**Tests**: Short-circuit behavior verification

**Estimated effort**: 0.5 context windows

---

#### Task 2.3.4: Threading Macros

**Description**: Implement threading for readability.

**Files to modify**:
- `lona/core/control.lona`

**Requirements**:
- `->` - thread first
- `->>` - thread last
- `as->` - named threading
- `some->` - nil-safe thread first
- `some->>` - nil-safe thread last
- `cond->`, `cond->>` - conditional threading

**Tests**: Full coverage with various forms

**Estimated effort**: 1-2 context windows

---

#### Task 2.3.5: Iteration Macros

**Description**: Implement looping constructs.

**Files to modify**:
- `lona/core/control.lona`

**Requirements**:
- `dotimes` - fixed iterations
- `doseq` - sequence iteration
- `for` - list comprehension
- `while` - condition loop

**Tests**: Full coverage

**Estimated effort**: 1-2 context windows

---

### Phase 2.4: Collection Functions

#### Task 2.4.0: Collection Transformation

**Description**: Implement collection converters. This task must come first as other collection functions depend on `into`.

**Files to create**:
- `lona/core/coll.lona`

**Requirements**:
- `into` - pour into collection: `(defn into [to from] (reduce conj to (seq from)))`
- `empty` - empty version of collection
- `vec` - to vector: `(defn vec [coll] (if (vector? coll) coll (into [] coll)))`
- `set` - to set: `(defn set [coll] (into #{} coll))`
- `sort`, `sort-by` - sorting

**Note**: `vec` and `set` are pure Lonala implementations using `into`, not native wrappers.

**Tests**: Full coverage

**Estimated effort**: 1 context window

---

#### Task 2.4.1: Collection Predicates

**Description**: Implement collection testing functions.

**Files to modify**:
- `lona/core/coll.lona`

**Requirements**:
- `empty?` - test if empty
- `not-empty` - nil if empty, else coll
- `every?` - all match predicate
- `some` - any match predicate
- `not-every?`, `not-any?` - negations

**Tests**: Full coverage

**Estimated effort**: 1 context window

---

#### Task 2.4.2: Collection Constructors

**Description**: Implement collection constructor functions in pure Lonala.

**Files to modify**:
- `lona/core/coll.lona`

**Requirements**:
- `list` - create list: `(defn list [& args] args)`
- `vector` - create vector: `(defn vector [& args] (into [] args))`
- `hash-map` - create map: `(defn hash-map [& kvs] (apply assoc {} kvs))`
- `hash-set` - create set: `(defn hash-set [& vals] (into #{} vals))`

**Dependencies**: Requires `apply` (Task 2.2.7), `into` (Task 2.4.0)

**Tests**:
- Constructor with zero arguments returns empty collection
- Constructor with multiple arguments
- Type verification of results

**Estimated effort**: 0.5 context windows

---

#### Task 2.4.3: Collection Analysis

**Description**: Implement collection analysis.

**Files to modify**:
- `lona/core/coll.lona`

**Requirements**:
- `frequencies` - count occurrences
- `group-by` - group by key function
- `distinct` - remove duplicates
- `dedupe` - remove consecutive duplicates

**Tests**: Full coverage

**Estimated effort**: 1 context window

---

#### Task 2.4.4: Map Functions

**Description**: Implement map-specific operations.

**Files to modify**:
- `lona/core/coll.lona`

**Requirements**:
- `merge` - combine maps
- `merge-with` - combine with function
- `select-keys` - subset of keys
- `rename-keys` - rename keys
- `map-keys`, `map-vals` - transform

**Tests**: Full coverage

**Estimated effort**: 1 context window

---

#### Task 2.4.5: Set Functions

**Description**: Implement set operations.

**Files to create**:
- `lona/set.lona`

**Requirements**:
- `union` - combine sets
- `intersection` - common elements
- `difference` - remove elements
- `subset?`, `superset?` - containment

**Tests**: Full coverage

**Estimated effort**: 1 context window

---

### Phase 2.5: Protocol System

Implement Clojure-style protocols for polymorphic dispatch.

> **Dependency**: Protocol dispatch uses native `type-of` (Task 1.8.21) for O(1) type lookup instead of O(N) predicate chains.

#### Task 2.5.1: Protocol Definition (`defprotocol`)

**Description**: Macro to define protocol with method signatures.

**Files to create**:
- `lona/core/protocol.lona`

**Requirements**:
- ```clojure
  (defprotocol BlockDevice
    (read-block [dev block-id])
    (write-block [dev block-id data]))
  ```
- Creates dispatch functions for each method
- Stores method signatures in protocol metadata
- Protocol is a first-class value (map with dispatch tables)

**Tests**:
- Define protocol
- Protocol has expected methods
- Dispatch functions created
- Calling before implementation errors

**Estimated effort**: 1-2 context windows

---

#### Task 2.5.2: Type-Based Extension (`extend-type`)

**Description**: Extend protocol to a specific type.

**Files to modify**:
- `lona/core/protocol.lona`

**Requirements**:
- ```clojure
  (extend-type VirtioBlockDevice
    BlockDevice
    (read-block [dev block-id] ...)
    (write-block [dev block-id data] ...))
  ```
- Registers implementations in protocol dispatch table
- Type checked at call time, dispatches to implementation
- Multiple protocols can be extended for same type

**Tests**:
- Extend protocol to type
- Dispatch works correctly
- Multiple types with same protocol
- Error on unimplemented

**Estimated effort**: 1-2 context windows

---

#### Task 2.5.3: Map-Based Extension (`extend`)

**Description**: Extend protocol using explicit map of implementations.

**Files to modify**:
- `lona/core/protocol.lona`

**Requirements**:
- ```clojure
  (extend VirtioBlockDevice
    BlockDevice
    {:read-block (fn [dev block-id] ...)
     :write-block (fn [dev block-id data] ...)})
  ```
- More flexible than `extend-type`
- Can extend to `nil` and other special cases

**Tests**:
- Extend with map
- nil extension works
- Override existing extension

**Estimated effort**: 1 context window

---

#### Task 2.5.4: Protocol Predicates and Introspection

**Description**: Query protocol satisfaction and implementations.

**Files to modify**:
- `lona/core/protocol.lona`

**Requirements**:
- `(satisfies? Protocol value)` - does value's type implement protocol?
- `(extends? Protocol Type)` - is type extended to protocol?
- `(extenders Protocol)` - list all types implementing protocol

**Tests**:
- satisfies? positive and negative
- extends? for extended and non-extended types
- extenders lists all

**Estimated effort**: 0.5 context windows

---

### Phase 2.6: String Functions

#### Task 2.6.1: String Basics

**Description**: Implement basic string operations.

**Files to create**:
- `lona/string.lona`

**Requirements**:
- `str` - concatenate to string
- `subs` - substring
- `count` - string length (polymorphic)
- `blank?` - empty or whitespace
- `join` - join with separator

**Tests**: Full coverage

**Estimated effort**: 1 context window

---

#### Task 2.6.2: String Transformation

**Description**: Implement string transformers.

**Files to modify**:
- `lona/string.lona`

**Requirements**:
- `upper-case`, `lower-case`
- `capitalize`
- `trim`, `triml`, `trimr`
- `replace`, `replace-first`
- `reverse`

**Tests**: Full coverage

**Estimated effort**: 1 context window

---

#### Task 2.6.3: String Analysis

**Description**: Implement string analysis.

**Files to modify**:
- `lona/string.lona`

**Requirements**:
- `split` - split by regex/string
- `split-lines` - split by newlines
- `includes?` - substring test
- `starts-with?`, `ends-with?`
- `index-of`, `last-index-of`

**Tests**: Full coverage

**Estimated effort**: 1 context window

---

### Phase 2.7: Numeric Functions

#### Task 2.7.1: Numeric Operations

**Description**: Implement numeric utilities.

**Files to create**:
- `lona/core/num.lona`

**Requirements**:
- `inc`, `dec` - increment/decrement
- `abs` - absolute value
- `min`, `max` - extremes
- `pos?`, `neg?`, `zero?` - sign tests
- `even?`, `odd?` - parity

**Tests**: Full coverage

**Estimated effort**: 1 context window

---

#### Task 2.7.2: Math Functions

**Description**: Implement math operations.

**Files to modify**:
- `lona/core/num.lona`

**Requirements**:
- `quot`, `rem` - quotient/remainder
- `pow` - exponentiation
- `sqrt` - square root
- `floor`, `ceil`, `round`

**Tests**: Full coverage

**Estimated effort**: 1 context window

---

### Phase 2.8: I/O Functions

#### Task 2.8.1: Print Functions

**Description**: Implement output functions.

**Files to create**:
- `lona/core/io.lona`

**Requirements**:
- `print` - print without newline
- `println` - print with newline
- `pr` - print readably
- `prn` - pr with newline
- `pr-str` - to string readably

**Tests**: Output verification

**Estimated effort**: 1 context window

---

#### Task 2.8.2: Read Functions

**Description**: Implement input functions.

**Files to modify**:
- `lona/core/io.lona`

**Requirements**:
- `read-string` - parse string to value
- `read` - read from input (requires I/O)

**Tests**: Round-trip tests

**Estimated effort**: 1 context window

---

### Phase 2.9: Process Functions

#### Task 2.9.1: Process Utilities

**Description**: Implement process helper functions.

**Files to create**:
- `lona/process.lona`

**Requirements**:
- `spawn` wrappers with options
- `call` - synchronous request/response
- `cast` - async request (no response)
- `reply` - send reply to caller

**Tests**: Process interaction tests

**Estimated effort**: 1-2 context windows

---

#### Task 2.9.2: GenServer Pattern

**Description**: Implement GenServer behavior.

**Files to modify**:
- `lona/process.lona`

**Requirements**:
- `defserver` macro
- `handle-call`, `handle-cast`, `handle-info`
- `init`, `terminate` callbacks
- State management

**Tests**: GenServer behavior tests

**Estimated effort**: 2 context windows

---

#### Task 2.9.3: Named Process Registry

**Description**: Implement named process registration.

**Files to modify**:
- `lona/process.lona`

**Requirements**:
- `register` - register process with name
- `unregister` - remove registration
- `whereis` - lookup process by name
- `registered` - list all registered names
- Automatic unregister on process exit

**Tests**: Registration lifecycle tests

**Estimated effort**: 1 context window

---

#### Task 2.9.4: Supervisor Behaviors

**Description**: Implement OTP-style supervision strategies.

**Files to create**:
- `lona/supervisor.lona`

**Requirements**:
- `def-supervisor` macro for declarative supervisors
- `:one-for-one` strategy (restart failed child only)
- `:one-for-all` strategy (restart all children on any failure)
- `:rest-for-one` strategy (restart failed child and all started after it)
- Child specifications with restart policies
- Maximum restart intensity (max restarts per time window)

**Tests**: Supervisor strategy tests, restart behavior tests

**Estimated effort**: 2-3 context windows

---

#### Task 2.9.5: Atom Watches and Validators

**Description**: Implement atom observation and validation.

**Files to modify**:
- `lona/core/atom.lona`

**Requirements**:
- `swap!` - update via function using CAS retry loop
- `add-watch` - register watcher function `(fn [key atom old new] ...)`
- `remove-watch` - unregister watcher
- `set-validator!` - set validation function (throws on invalid)
- All built on native `atom`, `deref`, `reset!`, `compare-and-set!`

**Implementation**:
```clojure
(defn swap! [a f & args]
  (loop []
    (let [old @a
          new (apply f old args)]
      (if (compare-and-set! a old new)
        new
        (recur)))))
```

**Tests**: CAS retry, watcher notification, validator rejection

**Estimated effort**: 1-2 context windows

---

### Phase 2.10: Error Handling

#### Task 2.10.1: Result Functions

**Description**: Implement result tuple utilities.

**Files to create**:
- `lona/result.lona`

**Requirements**:
- `ok?`, `error?` - predicates
- `unwrap!`, `unwrap-or`, `unwrap-error`
- `map-ok`, `map-error`
- `and-then`, `or-else`

**Tests**: Full coverage

**Estimated effort**: 1 context window

---

#### Task 2.10.2: Error Handling Macros

**Description**: Implement error handling conveniences.

**Files to modify**:
- `lona/result.lona`

**Requirements**:
- `with` - chain fallible operations
- `if-ok`, `when-ok`
- `ok->`, `ok->>`
- `assert!`

**Tests**: Full coverage

**Estimated effort**: 1-2 context windows

---

### Phase 2.11: Lazy Sequences

#### Task 2.11.1: LazySeq Type

**Description**: Implement lazy sequence foundation.

**Files to create**:
- `lona/core/lazy.lona`

**Requirements**:
- `lazy-seq` macro
- Lazy evaluation on access
- Caching of realized values
- Integration with seq functions

**Tests**: Laziness verification

**Estimated effort**: 2 context windows

---

#### Task 2.11.2: Lazy Versions of Seq Functions

**Description**: Make seq functions lazy.

**Files to modify**:
- `lona/core/seq.lona`
- `lona/core/lazy.lona`

**Requirements**:
- Lazy `map`, `filter`, `remove`
- Lazy `take`, `drop`, `take-while`
- Lazy `concat`, `mapcat`
- Force realization when needed

**Tests**: Laziness preservation tests

**Estimated effort**: 2 context windows

---

### Phase 2.12: Standard Library Tests

#### Task 2.12.1: Core Function Tests

**Description**: Complete test coverage for core.

**Files to create**:
- `test/core_test.lona`

**Requirements**:
- Tests for all seq functions
- Tests for all coll functions
- Tests for all control macros

**Estimated effort**: 2-3 context windows

---

#### Task 2.12.2: String and Numeric Tests

**Description**: Complete test coverage for strings and numbers.

**Files to create**:
- `test/string_test.lona`
- `test/numeric_test.lona`

**Requirements**:
- Tests for all string functions
- Tests for all numeric functions
- Edge case coverage

**Estimated effort**: 1-2 context windows

---

#### Task 2.12.3: Process and Result Tests

**Description**: Complete test coverage for process and error handling.

**Files to create**:
- `test/process_test.lona`
- `test/result_test.lona`

**Requirements**:
- Tests for process utilities
- Tests for GenServer
- Tests for result functions

**Estimated effort**: 1-2 context windows

---

### Phase 2.13: Self-Hosting

Enable Lonala to compile and evaluate itself at runtime.

#### Task 2.13.1: eval Function

**Description**: Implement runtime evaluation of Lonala forms.

**Files to create**:
- `lona/core/eval.lona`

**Requirements**:
- `eval` - evaluate a form at runtime
- Uses native `compile` and `vm/load` primitives
- Works with any valid Lonala form
- Proper namespace context handling

**Implementation**:
```clojure
(defn eval [form]
  (vm/load (compiler/compile form)))
```

**Tests**: Eval of various form types

**Estimated effort**: 1 context window

---

#### Task 2.13.2: load Function

**Description**: Implement file loading and evaluation.

**Files to modify**:
- `lona/core/eval.lona`

**Requirements**:
- `load` - read and evaluate file contents
- `load-string` - evaluate string as code
- Proper namespace handling during load
- Error reporting with file/line context

**Tests**: Load various file types, error handling

**Estimated effort**: 1-2 context windows

---

#### Task 2.13.3: Sorted Collection Subsequences

**Description**: Implement sorted collection range queries.

**Files to create**:
- `lona/core/sorted.lona`

**Requirements**:
- `subseq` - get subsequence from sorted collection matching condition
- `rsubseq` - get reverse subsequence matching condition
- Support `>`, `>=`, `<`, `<=` conditions
- Built using iteration over sorted structure

**Implementation**:
```clojure
(defn subseq
  ([sc test key]
   (filter #(test (first %) key) (seq sc)))
  ([sc start-test start-key end-test end-key]
   (filter #(and (start-test (first %) start-key)
                 (end-test (first %) end-key))
           (seq sc))))
```

**Tests**: Various range queries on sorted maps and sets

**Estimated effort**: 1 context window

---

