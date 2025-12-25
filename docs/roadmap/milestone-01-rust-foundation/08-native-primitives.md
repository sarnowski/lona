# Phase 1.8: Native Primitives

Implement remaining native functions from minimal-rust.md.

---

## Task 1.8.1: Type Predicates - Complete Set

**Description**: Implement all type predicate functions.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `nil?`, `boolean?`, `integer?`, `float?`, `ratio?`
- `symbol?`, `keyword?`, `string?`, `binary?`
- `list?`, `vector?`, `map?`, `set?`, `fn?`
- `coll?`, `seq?`

**Tests**:
- Each predicate returns correct result
- Cross-type testing

**Estimated effort**: 1 context window

---

## Task 1.8.2: Bitwise Operations

**Description**: Implement bitwise primitives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `bit-and`, `bit-or`, `bit-xor`, `bit-not`
- `bit-shift-left`, `bit-shift-right`
- Work on integers

**Tests**:
- Each operation
- Edge cases (negative numbers, large shifts)

**Estimated effort**: 1 context window

---

## Task 1.8.3: Collection Primitives - nth, count, conj

**Description**: Implement core collection accessors.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(nth coll idx)` - indexed access
- `(count coll)` - collection size
- `(conj coll item)` - add to collection

**Tests**:
- Each function on each collection type
- Edge cases (empty, out of bounds)

**Estimated effort**: 1 context window

---

## Task 1.8.4: Map Operations - get, assoc, dissoc, keys, vals

**Description**: Implement map manipulation primitives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(get m k)` - get value for key
- `(assoc m k v)` - add/update key
- `(dissoc m k)` - remove key
- `(keys m)`, `(vals m)` - key/value sequences

**Tests**:
- Each operation
- Missing key behavior
- Large maps

**Estimated effort**: 1 context window

---

## Task 1.8.5: Set Operations - disj, contains?

**Description**: Implement set manipulation primitives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(disj set elem)` - remove element
- `(contains? set elem)` - membership test

**Tests**:
- Each operation
- Missing element behavior

**Estimated effort**: 0.5 context windows

---

## Task 1.8.6: Callable Value Types (Keywords and Sets as Functions)

**Description**: Enable keywords and sets to be invoked as functions, following Clojure semantics.

**Dependencies**: Task 1.8.4 (Map Operations - `get`), Task 1.8.5 (Set Operations - `contains?`)

**Files to modify**:
- `crates/lona-kernel/src/vm/interpreter/mod.rs` (call dispatch)

**Requirements**:
- Keywords invoke `get` on the second argument: `(:a {:a 1})` → `1`
- Keywords with optional default: `(:a {} :not-found)` → `:not-found`
- Sets check membership, returning the element or nil: `(#{:a :b} :a)` → `:a`, `(#{:a :b} :c)` → `nil`
- Error on wrong arity (keywords require 1-2 args, sets require 1 arg)
- Error if keyword's first arg is not a map
- Error if set is called with wrong number of args

**Clojure semantics reference**:
```clojure
;; Keywords as functions (lookup in map)
(:name {:name "Alice" :age 30})     ; => "Alice"
(:missing {:name "Alice"})          ; => nil
(:missing {:name "Alice"} "default"); => "default"

;; Sets as functions (membership test, returns element or nil)
(#{:a :b :c} :a)                    ; => :a
(#{:a :b :c} :d)                    ; => nil
(#{1 2 3} 2)                        ; => 2
```

**Implementation approach**:
1. In the VM call dispatch, before erroring on non-function in call position, check if value is Keyword or Set
2. For Keyword: call the existing `get` native with (arg1, keyword, optional-default)
3. For Set: check if element is in set, return element or nil

**Tests**:
- Keyword lookup in map returns value
- Keyword lookup for missing key returns nil
- Keyword lookup with default returns default when missing
- Keyword lookup with default returns value when present
- Keyword called with non-map errors
- Keyword called with wrong arity errors
- Set membership returns element when present
- Set membership returns nil when absent
- Set called with wrong arity errors
- Works with nested structures: `(:a (:b {:b {:a 1}}))`

**Estimated effort**: 1 context window

---

## Task 1.8.7: Binary Operations

**Description**: Implement binary buffer primitives with ownership semantics.

**Dependencies**: Task 1.1.4 (Binary Value Type)

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:

*Creation and Query:*
- `(make-binary size)` - allocate zeroed buffer, returns Owned binary
- `(binary-len buf)` - get length (works on Owned or View)
- `(binary-owner? buf)` - returns true if buf is Owned

*Access:*
- `(binary-get buf idx)` - get byte at index (works on Owned or View)
- `(binary-set buf idx byte)` - set byte (Owned only, error on View)

*Slicing and Views:*
- `(binary-slice buf start len)` - zero-copy slice, inherits access mode
- `(binary-view buf)` - create read-only view of buffer

*Copying:*
- `(binary-copy! dst dst-off src src-off len)` - copy bytes (dst must be Owned)

*Ownership Transfer:*
- `(binary-transfer! pid buf)` - transfer ownership to another process
  - Requires: buf is Owned, no other references exist (Rc::strong_count == 1)
  - Effect: buf becomes zombie (operations error), recipient gets Owned binary
  - Error on View, error if references exist

**Ownership Semantics in Operations**:

| Operation | Owned | View | Zombie |
|-----------|-------|------|--------|
| `binary-len` | ✓ | ✓ | error |
| `binary-get` | ✓ | ✓ | error |
| `binary-set` | ✓ | error | error |
| `binary-slice` | → Owned | → View | error |
| `binary-view` | → View | → View | error |
| `binary-copy!` (as dst) | ✓ | error | error |
| `binary-copy!` (as src) | ✓ | ✓ | error |
| `binary-transfer!` | ✓* | error | error |

*Transfer only succeeds if `Rc::strong_count == 1`

**Message Passing Behavior**:
When a Binary is sent via `(send pid msg)`:
- Runtime automatically converts to View (not transfer)
- Original remains Owned in sender
- Use `binary-transfer!` for explicit ownership transfer

**Tests**:
- make-binary returns Owned
- binary-get/set with Owned
- binary-set with View (error)
- binary-slice preserves access mode
- binary-view creates View
- binary-copy! with Owned dst
- binary-copy! with View dst (error)
- binary-transfer! success case
- binary-transfer! with View (error)
- binary-transfer! with outstanding refs (error)
- Operations on zombie binary (error)
- Bounds checking on all operations

**Estimated effort**: 2 context windows

---

## Task 1.8.8: Symbol Operations

**Description**: Implement symbol creation primitives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(symbol name)` - create/intern symbol
- `(gensym)` - unique symbol (already done in 1.2.10)
- `(name sym)` - get symbol name as string

**Tests**:
- Symbol creation
- Interning verification
- Name extraction

**Estimated effort**: 0.5 context windows

---

## Task 1.8.9: Metadata Operations

**Description**: Implement metadata primitives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(meta obj)` - get metadata
- `(with-meta obj m)` - return copy with metadata
- `(vary-meta obj f args...)` - transform metadata

**Tests**:
- Get/set metadata
- vary-meta transformation
- Unsupported types return nil

**Estimated effort**: 1 context window

---

## Task 1.8.10: MMIO Primitives

**Description**: Implement memory-mapped I/O for drivers.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-runtime/src/platform/mmio.rs` (new)

**Requirements**:
- `(mmio-read-u8 cap offset)` - read u8 at offset within capability range
- `(mmio-read-u16 cap offset)` - read u16 at offset within capability range
- `(mmio-read-u32 cap offset)` - read u32 at offset within capability range
- `(mmio-read-u64 cap offset)` - read u64 at offset within capability range
- `(mmio-write-u8 cap offset val)` - write u8 at offset within capability range
- `(mmio-write-u16 cap offset val)` - write u16 at offset within capability range
- `(mmio-write-u32 cap offset val)` - write u32 at offset within capability range
- `(mmio-write-u64 cap offset val)` - write u64 at offset within capability range
- Capability is an explicit first argument (no ambient authority)
- Offset is validated against capability's address range

**Design Note**: Following "No Ambient Authority" principle from `docs/development/principles.md`. Capabilities are explicit inputs, not implicit domain permissions.

**Tests**:
- Read/write operations (mock)
- Capability enforcement (invalid cap rejected)
- Offset bounds checking

**Estimated effort**: 1-2 context windows

---

## Task 1.8.11: DMA Primitives

**Description**: Implement DMA buffer management.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-runtime/src/platform/dma.rs` (new)

**Requirements**:
- `(dma-alloc dma-cap size)` - allocate DMA-capable buffer using DMA pool capability
- `(dma-free dma-cap buffer)` - free DMA buffer
- `(phys-addr buffer)` - get physical address of DMA buffer
- `(memory-barrier)` - ensure ordering (no capability needed, CPU-local operation)
- DMA allocation requires explicit DMA pool capability

**Design Note**: Following "No Ambient Authority" principle. Domain must hold DMA pool capability to allocate DMA buffers.

**Tests**:
- Allocation with valid capability
- Rejection without DMA capability
- Physical address retrieval
- Barrier execution

**Estimated effort**: 1-2 context windows

---

## Task 1.8.12: IRQ Primitives

**Description**: Implement interrupt handling for drivers.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-runtime/src/platform/irq.rs` (new)

**Requirements**:
- `(irq-wait cap)` - block until interrupt
- IRQ wakes process via scheduler
- Requires IRQ capability

**Tests**:
- IRQ wait (mock)
- Process wake on IRQ

**Estimated effort**: 1-2 context windows

---

## Task 1.8.13: Time Primitives

**Description**: Implement time-related functions.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/scheduler/timer.rs`

**Requirements**:
- `(now-ms)` - current time in milliseconds
- `(send-after pid delay msg)` - delayed message

**Tests**:
- Time retrieval
- Delayed message delivery

**Estimated effort**: 1 context window

---

## Task 1.8.14: Atom Primitives

**Description**: Implement process-local mutable state.

**Files to modify**:
- `crates/lona-core/src/value/atom.rs` (new)
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lonala-parser/src/parser/mod.rs` (for `@` reader macro)

**Requirements**:
- `(atom val)` - create atom
- `(deref a)` - get current value
- `(reset! a val)` - set value
- `(compare-and-set! a old new)` - CAS
- **Parser**: Add `@` reader macro that expands `@a` to `(deref a)`

**Tests**:
- Atom creation
- Get/set operations
- CAS semantics
- `@` reader macro expansion

**Estimated effort**: 1-2 context windows

---

## Task 1.8.15: Sorted Collections - Basic

**Description**: Implement sorted map and set.

**Files to modify**:
- `crates/lona-core/src/value/sorted.rs` (new)
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(sorted-map & kvs)` - sorted by key
- `(sorted-set & vals)` - sorted elements
- Iteration in sorted order

**Tests**:
- Creation
- Sorted iteration
- Operations maintain order

**Estimated effort**: 2 context windows

---

## Task 1.8.16: Sorted Collections - Custom Comparators

**Description**: Add custom comparator support.

**Files to modify**:
- `crates/lona-core/src/value/sorted.rs`
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(sorted-map-by cmp & kvs)` - custom comparator
- `(sorted-set-by cmp & vals)` - custom comparator
- Comparator is a function

**Tests**:
- Custom comparator
- Reverse order
- Complex comparisons

**Estimated effort**: 1 context window

---

## Task 1.8.17: Regular Expressions - Compilation

**Description**: Implement regex pattern compilation.

**Files to modify**:
- `crates/lona-core/src/value/regex.rs` (new)
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lonala-parser/src/parser/mod.rs`

**Requirements**:
- `(re-pattern str)` - compile regex
- `#"pattern"` reader syntax
- Use Rust regex crate (no_std compatible)

**Tests**:
- Pattern compilation
- Reader syntax
- Invalid pattern error

**Estimated effort**: 1-2 context windows

---

## Task 1.8.18: Regular Expressions - Matching

**Description**: Implement regex matching functions.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(re-find re str)` - first match
- `(re-matches re str)` - full string match
- `(re-seq re str)` - all matches
- Group extraction

**Tests**:
- Find operations
- Match operations
- Group capture
- No match cases

**Estimated effort**: 1-2 context windows

---

## Task 1.8.19: String Primitive Operations

**Description**: Core string operations requiring native implementation for UTF-8 handling.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-core/src/value/string.rs`

**Requirements**:
- `(string-length s)` - length in characters (not bytes)
- `(codepoint-at s idx)` - Unicode codepoint at index (returns **Integer**, not String)
- `(subs s start)` and `(subs s start end)` - substring
- Proper UTF-8 handling (index by codepoint, not byte)
- Error on invalid index
- Handle multi-byte characters correctly (emoji, CJK, etc.)

> **CHANGED**: `codepoint-at` returns an **Integer** (Unicode codepoint) instead of a single-character String. This avoids heap allocation during string iteration. Use `(char->string codepoint)` in Lonala when a String is needed.

**Tests**:
- Length of ASCII string
- Length of Unicode string (emoji, etc.)
- codepoint-at various positions (verify returns integer)
- Substring extraction
- UTF-8 boundary handling
- Error on out-of-bounds index

**Estimated effort**: 1-2 context windows

---

## Task 1.8.20: `apply` Native Primitive (CRITICAL)

**Unblocks**: This task unblocks the `vary-meta` metadata primitive.
`vary-meta` requires `apply` for its `(vary-meta obj f & args)` signature.
After `apply` is implemented, implement `vary-meta` as well.

**Description**: Implement `apply` for calling functions with runtime argument lists.

**Files to modify**:
- `crates/lona-core/src/opcode/mod.rs`
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- `(apply f args)` - call f with elements of args as arguments
- New opcode `APPLY` that pops function and arg list, then invokes
- Works with any callable (functions, closures, multi-arity)
- Args must be a list/vector/seq

> **Why Native**: `apply` cannot be implemented in Lonala because the argument list length is only known at runtime. A macro cannot handle runtime lists, and there's no way to dynamically construct a function call without VM support.

**Tests**:
- Apply with empty args
- Apply with multiple args
- Apply with multi-arity function
- Apply with variadic function
- Error on non-callable
- Error on non-sequence args

**Estimated effort**: 1-2 context windows

---

## Task 1.8.21: `type-of` Native Primitive (CRITICAL)

**Description**: Return the type of a value as a keyword.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(type-of x)` returns one of: `:nil`, `:boolean`, `:integer`, `:float`, `:ratio`, `:symbol`, `:keyword`, `:string`, `:binary`, `:list`, `:vector`, `:map`, `:set`, `:function`, `:atom`, `:regex`
- O(1) operation (check internal type tag)
- Used by protocol system for efficient dispatch

> **Why Native**: Type tags are internal to the VM. This enables O(1) protocol dispatch instead of O(N) predicate chains.

**Tests**:
- All value types return correct keyword
- Consistent across equal values

**Estimated effort**: 0.5 context windows

---

## Task 1.8.22: `identical?` Native Primitive

**Description**: Reference equality check.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(identical? x y)` returns true iff x and y are the same object in memory
- Different from `=` which checks structural equality
- For immutable primitives (integers, keywords), identity may equal value equality
- For collections, only true if same reference

**Tests**:
- Same object is identical
- Equal but different objects are not identical
- Keywords/symbols with same name are identical (interned)
- Two equal vectors are not identical

**Estimated effort**: 0.5 context windows

---

## Task 1.8.23: `native-print` Bootstrap Primitive (CRITICAL)

**Description**: Temporary print function for Milestone 2 bootstrap.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-runtime/src/platform/uart.rs`

**Requirements**:
- `(native-print x)` - outputs string representation to Rust internal UART
- Handles all value types (uses Rust Debug formatting)
- Returns nil
- **Temporary**: Will be deprecated when Lonala UART driver (M3) is complete

> **Why Needed**: M2 Test Framework needs to output results, but M3 (UART driver) depends on M2. This breaks the circular dependency.

**Tests**:
- Print string
- Print number
- Print collection
- Print nil

**Estimated effort**: 0.5 context windows

---

## Task 1.8.24: `string-concat` Native Primitive

**Description**: Concatenate two strings.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-core/src/value/string.rs`

**Requirements**:
- `(string-concat s1 s2)` - returns new string with s1 followed by s2
- Both arguments must be strings
- Allocates new string (immutable strings)

> **Why Native**: String allocation requires runtime memory management.

**Tests**:
- Concat two strings
- Concat with empty string
- Concat unicode strings
- Error on non-strings

**Estimated effort**: 0.5 context windows

---

## Task 1.8.25: `read-string` Native Primitive

**Description**: Parse a string into a Lonala value.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(read-string s)` - parses s and returns the first form
- Returns error tuple on parse failure
- Uses the Rust parser internally

> **Why Native**: The parser is implemented in Rust. This enables REPL and `eval` in Lonala.

**Tests**:
- Parse simple values (numbers, strings, keywords)
- Parse collections
- Parse nested forms
- Parse error handling

**Estimated effort**: 1 context window

---

## Task 1.8.26: `seq` Native Primitive

**Description**: Coerce a value to a sequence.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(seq nil)` returns nil
- `(seq ())` returns nil (empty list)
- `(seq non-empty-list)` returns the list
- `(seq vector)` returns list of elements
- `(seq map)` returns list of [key value] pairs
- `(seq set)` returns list of elements
- `(seq string)` returns list of codepoints (integers)

> **Why Native**: Needs to iterate internal data structures of vectors, maps, sets.

**Tests**:
- seq on nil
- seq on empty/non-empty list
- seq on vector
- seq on map (returns kv pairs)
- seq on set
- seq on string

**Estimated effort**: 1 context window

---

## Task 1.8.27: x86 Port I/O Primitives

**Description**: Implement x86-specific port I/O for devices that use I/O ports instead of MMIO.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-runtime/src/platform/port_io.rs` (new)

**Requirements**:
- `(port-in-u8 cap port)` - read byte from I/O port using capability
- `(port-in-u16 cap port)` - read 16-bit from I/O port
- `(port-in-u32 cap port)` - read 32-bit from I/O port
- `(port-out-u8 cap port val)` - write byte to I/O port
- `(port-out-u16 cap port val)` - write 16-bit to I/O port
- `(port-out-u32 cap port val)` - write 32-bit to I/O port
- Uses x86 `in`/`out` instructions
- Capability is explicit first argument; port validated against capability range

> **Platform-specific**: Only available on x86/x86_64. ARM uses MMIO exclusively.

**Design Note**: Following "No Ambient Authority" principle - I/O port capability is explicit.

**Tests**:
- Port read/write operations (mock on non-x86)
- Capability enforcement

**Estimated effort**: 1 context window

---

## Task 1.8.28: Bundled Source Loading

**Description**: Implement primitives for loading Lonala source bundled with the runtime image.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-runtime/src/bundled.rs` (new)

**Requirements**:
- `(bundled-sources)` - return list of bundled source file names
- `(bundled-read name)` - read bundled source as string
- Bundled sources compiled into runtime at build time
- Used by `load` function before filesystem is available (Milestone 7)

**Design Note**: Enables Milestone 2's `load` function to work before filesystem. Standard library sources are bundled into the runtime image.

**Tests**:
- List bundled sources
- Read known bundled source
- Error on unknown name

**Estimated effort**: 1 context window
