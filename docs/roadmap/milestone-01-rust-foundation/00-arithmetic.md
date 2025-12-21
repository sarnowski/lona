## Phase 1.0: Arithmetic Primitives

Arithmetic must come first — nearly everything else depends on it.

---

### Task 1.0.1: Native Addition and Subtraction

**Description**: Implement `+` and `-` in the VM natives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/vm/numeric.rs`

**Requirements**:
- `+` with zero args returns 0
- `+` with one arg returns that arg
- `+` with multiple args sums them
- `-` with one arg negates it
- `-` with multiple args subtracts subsequent from first
- Handle Integer, Float, and Ratio combinations
- Proper type promotion (Int+Float→Float, Int+Ratio→Ratio)

**Tests**:
- All type permutations (Int+Int, Int+Float, Ratio+Int, etc.)
- Edge cases (overflow to bigint, negative numbers)
- Zero and one argument cases

**Estimated effort**: 1 context window

---

### Task 1.0.2: Native Multiplication and Division

**Description**: Implement `*` and `/` in the VM natives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/vm/numeric.rs`

**Requirements**:
- `*` with zero args returns 1
- `*` with one arg returns that arg
- `/` with one arg returns reciprocal (1/x)
- Division of integers produces Ratio when inexact
- Division by zero handling (error tuple or NaN for floats)
- Handle all numeric type combinations

**Tests**:
- All type permutations
- Exact division (6/2 → 3 Integer)
- Inexact division (5/2 → 5/2 Ratio)
- Division by zero
- Reciprocal cases

**Estimated effort**: 1 context window

---

### Task 1.0.3: Modulo

**Description**: Implement `mod` native.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `mod` returns remainder of division
- Float modulo behavior matches IEEE 754
- Ratio modulo support

**Tests**:
- Integer modulo
- Negative number modulo
- Float modulo behavior

**Estimated effort**: 0.5 context windows

> **Note**: `inc` and `dec` are NOT native—they are trivially implemented in Lonala as `(defn inc [x] (+ x 1))` and `(defn dec [x] (- x 1))`. See Phase 2.7 Numeric Functions.

---

### Task 1.0.4: Comparison - Equality

**Description**: Implement `=` for value equality.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-core/src/value/mod.rs`

**Requirements**:
- Deep structural equality for collections
- Numeric equality across types (1 = 1.0 = 1/1)
- `=` with multiple args checks all pairwise
- NaN is not equal to anything (including itself)
- Symbol equality by identity (interned)

**Tests**:
- All primitive types
- Nested collections
- Mixed numeric types
- NaN behavior
- Multiple argument form

**Estimated effort**: 1 context window

---

### Task 1.0.5: Comparison - Ordering

**Description**: Implement `<`, `>`, `<=`, `>=` natives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- Work on all numeric types
- Cross-type comparison (1 < 1.5 < 2)
- String comparison (lexicographic)
- Multiple args: `(< a b c)` means a < b < c
- Error on non-comparable types

**Tests**:
- All numeric types
- Cross-type comparison
- String comparison
- Multiple argument chaining
- Error cases

**Estimated effort**: 1 context window
