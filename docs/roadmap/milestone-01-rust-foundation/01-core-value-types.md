# Phase 1.1: Core Value Type Extensions

Extend the value system with missing fundamental types.

---

## Task 1.1.1: Keyword Value Type

**Description**: Add `Keyword` as a distinct value type with interning.

**Files to modify**:
- `crates/lona-core/src/value/mod.rs`
- `crates/lona-core/src/value/keyword.rs` (new)
- `crates/lonala-parser/src/parser/mod.rs`

**Requirements**:
- Keywords are interned (like symbols) for fast equality
- Keywords are self-evaluating (evaluate to themselves)
- Keywords can be used as map keys
- Support qualified keywords (`:ns/name`)
- Parser produces `Value::Keyword` for `:foo` syntax

**Tests**:
- Keyword creation and interning
- Keyword equality (interned comparison)
- Keywords as map keys
- Keyword self-evaluation
- Qualified keyword parsing

**Estimated effort**: 1 context window

---

## Task 1.1.2: Set Value Type

**Description**: Add `Set` as a persistent hash set using HAMT.

**Files to modify**:
- `crates/lona-core/src/value/mod.rs`
- `crates/lona-core/src/value/set.rs` (new)
- `crates/lona-core/src/hamt/` (extend for sets)
- `crates/lonala-parser/src/parser/mod.rs`

**Requirements**:
- Immutable persistent set with structural sharing
- O(log32 n) for insert, remove, contains
- Parser supports `#{1 2 3}` literal syntax
- Duplicate detection in literals (error on `#{1 1}`)
- Set equality is structural

**Tests**:
- Set creation from literal
- Set operations (conj, disj, contains?)
- Set equality
- Duplicate detection in literals
- Empty set `#{}`

**Estimated effort**: 1-2 context windows

---

## Task 1.1.3: Collection Literal Syntax

**Description**: Implement compiler support for vector, map, and set literal syntax.

**Dependencies**: Task 1.1.1 (Keywords for map keys), Task 1.1.2 (Set type)

**Files to modify**:
- `crates/lonala-parser/src/lexer/mod.rs` (add `#` dispatch character)
- `crates/lonala-compiler/src/compiler/mod.rs` (emit bytecode for literals)

**Requirements**:
- `[1 2 3]` compiles to vector construction
- `{:a 1 :b 2}` compiles to hash-map construction
- `#{1 2 3}` compiles to hash-set construction (requires lexer update for `#` prefix)
- Empty literals: `[]`, `{}`, `#{}` all work
- Nested literals: `[{:a [1 2]}]`

**Tests**:
- Vector literal creates vector
- Map literal creates hash-map
- Set literal creates hash-set
- Empty collection literals
- Nested collection literals
- Duplicate key in map literal (error or last-wins)
- Duplicate element in set literal (error)

**Estimated effort**: 1-2 context windows

---

## Task 1.1.4: Binary Value Type

**Description**: Add `Binary` for raw byte buffers with ownership semantics, the ONLY mutable type in Lonala.

**Files to modify**:
- `crates/lona-core/src/binary.rs` (new)
- `crates/lona-core/src/lib.rs`
- `crates/lona-core/src/value/mod.rs`
- `crates/lona-core/src/value/display.rs`

**Design**: Binary implements an ownership model for safe concurrent access:

```
Binary Ownership Model:

┌─────────────────────────────────────────────────────────────────┐
│ BinaryBuffer (shared, reference-counted)                        │
│ ├── data: Option<Vec<u8>>   # None = transferred (zombie state)│
│ └── phys_addr: Option<u64>  # For DMA (populated by dma-alloc) │
└─────────────────────────────────────────────────────────────────┘
         ▲                    ▲
         │ Owned              │ View (read-only)
         │                    │
┌────────┴────────┐   ┌───────┴───────┐
│ Binary (owner)  │   │ Binary (view) │
│ - can read      │   │ - can read    │
│ - can write     │   │ - cannot write│
│ - can transfer  │   │ - cannot xfer │
│ - can make view │   │ - can make view│
└─────────────────┘   └───────────────┘
```

**Key semantics**:
- **Owned**: Created by `make-binary` or receiving `transfer!`. Full read/write access.
- **View**: Created by `binary-view` or cloning. Read-only access.
- **Clone**: Always produces View (prevents dual ownership).
- **Slice**: Inherits access mode from parent (`binary-slice` of Owned is Owned).
- **Transfer**: Blocked if `Rc::strong_count > 1` (other references exist). Sets buffer to zombie state (`data = None`), invalidating old references.
- **No locking**: No read/write synchronization. Concurrent access is programmer responsibility (raw performance for drivers).
- **Fixed size**: Binaries cannot be resized (prevents DMA address invalidation).

**Requirements**:
- `BinaryAccess` enum: `Owned` | `View`
- `BinaryBuffer` struct with `Option<Vec<u8>>` for zombie state
- `Binary` struct with `Rc<RefCell<BinaryBuffer>>`, access mode, offset, len
- Clone always produces View
- Methods: `new`, `len`, `is_empty`, `get`, `set`, `slice`, `view`, `as_slice`
- Display format: `#<binary:len owned|view>`
- PartialEq based on content (when not zombie)
- No Hash (panic if attempted - mutable types shouldn't be map keys)

**Tests**:
- Binary creation with size (zeroed)
- Byte get/set operations (Owned succeeds, View fails)
- Bounds checking
- View creation and access restrictions
- Slice with inherited access (Owned→Owned, View→View)
- Clone produces View
- Transfer succeeds when no other references
- Transfer fails with outstanding views (Rc::strong_count > 1)
- Zombie state: operations on transferred binary error
- Content equality
- Empty binary edge case
- Display format

**Estimated effort**: 1-2 context windows

---

## Task 1.1.5: Metadata System - Value Storage

**Description**: Add optional metadata map storage to values that support it.

**Files to modify**:
- `crates/lona-core/src/value/mod.rs`
- `crates/lona-core/src/value/list.rs`
- `crates/lona-core/src/value/vector.rs`
- `crates/lona-core/src/value/map.rs`
- `crates/lona-core/src/value/symbol.rs`

**Requirements**:
- List, Vector, Map, Symbol can carry metadata
- Metadata is a Map (or nil)
- Metadata does NOT affect equality or hash
- `with-meta` creates new value with metadata
- `meta` retrieves metadata (or nil)

**Tests**:
- Attach metadata to each supported type
- Metadata doesn't affect equality
- Metadata doesn't affect hash
- Nested metadata
- nil metadata is valid

**Estimated effort**: 1-2 context windows

---

## Task 1.1.6: Metadata System - Reader Syntax

**Description**: Add parser support for `^` metadata reader macro.

**Files to modify**:
- `crates/lonala-parser/src/lexer/mod.rs`
- `crates/lonala-parser/src/parser/mod.rs`

**Requirements**:
- `^{:key val}` attaches metadata map to next form
- `^:keyword` is shorthand for `^{:keyword true}`
- `^Type` is shorthand for `^{:tag Type}`
- Multiple metadata items merge: `^:a ^:b x` → `^{:a true :b true} x`

**Tests**:
- Full metadata map syntax
- Keyword shorthand
- Type tag shorthand
- Multiple metadata merge
- Metadata on various form types

**Estimated effort**: 1 context window

---

## Task 1.1.7: Metadata System - Compiler Integration

**Description**: Integrate metadata into compilation and var definitions.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/mod.rs`
- `crates/lona-kernel/src/vm/globals.rs`

**Requirements**:
- `def` with docstring sets `:doc` metadata
- Compiler tracks `:file`, `:line`, `:column` for definitions
- `defmacro` sets `:macro true` on var
- Var metadata separate from value metadata

**Tests**:
- Docstring becomes `:doc` metadata
- Source location tracking
- Macro metadata flag
- Var vs value metadata distinction

**Estimated effort**: 1 context window

**Note**: See `PLAN.md` in the repository root for the detailed implementation plan with phases.
