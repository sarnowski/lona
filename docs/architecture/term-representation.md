# Term Representation

This document specifies how Lonala values are represented in memory at the VM level. The design follows BEAM's proven tagged-word approach for memory efficiency and GC compatibility.

> **Note**: All code examples in this document are **pseudocode** for illustration purposes.

## Overview

Lonala uses a **tagged word** representation where type information is encoded in the low bits of a machine word. This enables:

- **Immediate values** (nil, booleans, small integers, symbols, keywords) in a single 8-byte word
- **Heap pointers** with type tags for boxed values
- **Uniform header words** on all heap objects for GC
- **Efficient type dispatch** without pointer dereference

```
TERM REPRESENTATION OVERVIEW
════════════════════════════════════════════════════════════════════════════════

IMMEDIATE (fits in one word):
┌────────────────────────────────────────────────────────────────────────┐
│ nil, true, false, small int, symbol ID, keyword ID                     │
│ ────────────────────────────────────────────────────────── 8 bytes     │
└────────────────────────────────────────────────────────────────────────┘

BOXED (pointer to heap object):
┌────────────────────────────────────────────────────────────────────────┐
│ Tagged pointer ──────────────────────────────────────────► Heap Object │
│ ────────────────────────────────────────────────────────── 8 bytes     │
└────────────────────────────────────────────────────────────────────────┘
                                                                    │
                                                                    ▼
                                              ┌──────────────────────────┐
                                              │ HEADER (type + size)     │
                                              ├──────────────────────────┤
                                              │ Data words...            │
                                              └──────────────────────────┘
```

---

## Tagged Words

A **Term** is an 8-byte (64-bit) value. The low 2 bits encode the primary tag:

```
TAGGED WORD FORMAT (64-bit)
════════════════════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────┬──────┐
│                     Payload (62 bits)                           │ Tag  │
│                                                                 │ (2b) │
└─────────────────────────────────────────────────────────────────┴──────┘
                                                                   ▲
                                                                   │
                                                              Bits 0-1

Primary Tags:
  00 = HEADER    - Only on heap, marks start of heap object
  01 = LIST      - Pointer to pair (pair)
  10 = BOXED     - Pointer to heap object with header
  11 = IMMEDIATE - Value encoded directly in word
```

### Why Low Bits?

Heap objects are always aligned to 8 bytes (or more). This means valid heap pointers always have their low 3 bits as zero. We "borrow" 2 of these bits for the tag, which costs nothing—we simply mask them off when dereferencing.

```
// Pseudocode: Extract pointer from tagged word
fn to_pointer(term: u64) -> *const u8 {
    (term & !0b11) as *const u8  // Mask off low 2 bits
}

// Pseudocode: Check primary tag
fn primary_tag(term: u64) -> u8 {
    (term & 0b11) as u8
}
```

---

## Immediate Values

When the primary tag is `11` (IMMEDIATE), the value is encoded directly in the word. A secondary tag in bits 2-3 distinguishes immediate types:

```
IMMEDIATE FORMAT
════════════════════════════════════════════════════════════════════════════════

┌───────────────────────────────────────────────────────┬────────┬──────┐
│                   Value (60 bits)                     │SubTag  │  11  │
│                                                       │ (2b)   │      │
└───────────────────────────────────────────────────────┴────────┴──────┘
                                                         Bits 2-3  Bits 0-1

Subtags (bits 2-3 when primary = 11):
  00 = SMALL_INT   - 60-bit signed integer
  01 = SYMBOL      - Interned symbol (index into realm table)
  10 = KEYWORD     - Interned keyword (index into realm table)
  11 = SPECIAL     - nil, true, false, and other special values
```

### Small Integers

Small integers use 60 bits for the value (sign-extended), giving a range of approximately ±2^59:

```
SMALL INTEGER
════════════════════════════════════════════════════════════════════════════════

┌───────────────────────────────────────────────────────┬────────┬──────┐
│              Signed Integer Value (60 bits)           │   00   │  11  │
└───────────────────────────────────────────────────────┴────────┴──────┘

Range: -576,460,752,303,423,488 to +576,460,752,303,423,487
       (approximately ±5.76 × 10^17)

Encoding:
  value = (term >> 4) as i64  // Arithmetic shift preserves sign

Examples:
  0   → 0x0000_0000_0000_0003
  1   → 0x0000_0000_0000_0013
  -1  → 0xFFFF_FFFF_FFFF_FFF3
  42  → 0x0000_0000_0000_02A3
```

Integers outside this range are promoted to **Bignum** (heap-allocated).

### Symbols and Keywords

Symbols and keywords are **interned** in a per-realm table. The term contains an index:

```
SYMBOL / KEYWORD
════════════════════════════════════════════════════════════════════════════════

┌───────────────────────────────────────────────────────┬────────┬──────┐
│                  Table Index (60 bits)                │ 01/10  │  11  │
└───────────────────────────────────────────────────────┴────────┴──────┘

Symbol:  subtag = 01, index into realm.symbol_table
Keyword: subtag = 10, index into realm.keyword_table

Interning ensures:
  - Symbol/keyword identity via pointer equality (index comparison)
  - Efficient hashing (just compare indices)
  - Compact representation (no string storage in term)
```

### Special Values

Special values use subtag `11` with a tertiary tag in bits 4-7:

```
SPECIAL VALUES
════════════════════════════════════════════════════════════════════════════════

┌───────────────────────────────────────────┬────────────┬────────┬──────┐
│              (unused, zero)               │ Tertiary   │   11   │  11  │
│                                           │  (4 bits)  │        │      │
└───────────────────────────────────────────┴────────────┴────────┴──────┘
                                              Bits 4-7    Bits 2-3  Bits 0-1

Tertiary tags:
  0000 = NIL
  0001 = TRUE
  0010 = FALSE
  0011 = UNBOUND (uninitialized var sentinel)

Encodings:
  nil   → 0x0000_0000_0000_000F
  true  → 0x0000_0000_0000_001F
  false → 0x0000_0000_0000_002F
```

---

## List (Pair) Pointers

When the primary tag is `01` (LIST), the term points to a **pair** (pair):

```
LIST POINTER
════════════════════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────┬──────┐
│                  Pointer to Pair (aligned)                      │  01  │
└─────────────────────────────────────────────────────────────────┴──────┘
                                    │
                                    ▼
                    ┌────────────────────────────────────┐
                    │ HEAD (first) - 8 bytes (Term)      │
                    ├────────────────────────────────────┤
                    │ REST (tail) - 8 bytes (Term)       │
                    └────────────────────────────────────┘

                    Total: 16 bytes
                    NO HEADER - pairs are headerless
```

**Key property**: Pairs have **no header word**. This is a deliberate optimization following BEAM—lists are extremely common, and saving 8 bytes per pair adds up quickly.

The GC identifies pairs by their primary tag (`01`), not by a header. This is safe because:
- All terms know their own type via the tag
- The GC follows pointers, not headers, for pairs
- Pairs are always exactly 16 bytes (2 terms)

---

## Boxed Pointers

When the primary tag is `10` (BOXED), the term points to a heap object with a header:

```
BOXED POINTER
════════════════════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────┬──────┐
│                  Pointer to Heap Object (aligned)               │  10  │
└─────────────────────────────────────────────────────────────────┴──────┘
                                    │
                                    ▼
                    ┌────────────────────────────────────┐
                    │ HEADER WORD (type + arity/size)    │ ← 8 bytes
                    ├────────────────────────────────────┤
                    │ Data word 0                        │
                    │ Data word 1                        │
                    │ ...                                │
                    └────────────────────────────────────┘
```

---

## Header Words

Every heap object (except pairs) starts with a **header word**. Headers have primary tag `00`:

```
HEADER WORD FORMAT
════════════════════════════════════════════════════════════════════════════════

┌───────────────────────────────────────────┬────────────────────┬──────┐
│              Arity / Size (54 bits)       │   Object Tag (8b)  │  00  │
└───────────────────────────────────────────┴────────────────────┴──────┘
                                              Bits 2-9              Bits 0-1

Object Tags (bits 2-9):
  0x00 = TUPLE       - Fixed-size indexed sequence
  0x01 = VECTOR      - Persistent vector (with capacity)
  0x02 = MAP         - Key-value map
  0x03 = STRING      - UTF-8 byte sequence
  0x04 = BINARY      - Raw byte sequence
  0x05 = BIGNUM      - Arbitrary precision integer
  0x06 = FLOAT       - 64-bit IEEE 754
  0x07 = FUN         - Compiled function
  0x08 = CLOSURE     - Function with captured environment
  0x09 = PID         - Process identifier
  0x0A = REF         - Unique reference
  0x0B = PROCBIN     - Reference to large binary (off-heap)
  0x0C = SUBBIN      - Sub-binary view into existing binary
  ...
  0xFF = FORWARD     - Forwarding pointer (GC only)

Arity/Size interpretation depends on object tag:
  - TUPLE: number of elements
  - STRING/BINARY: byte length
  - BIGNUM: number of limbs
  - FUN: bytecode length + constant count
  - etc.
```

### Header Encoding

```
// Pseudocode: Create header word
fn make_header(object_tag: u8, arity: u64) -> u64 {
    (arity << 10) | ((object_tag as u64) << 2) | 0b00
}

// Pseudocode: Extract object tag
fn header_object_tag(header: u64) -> u8 {
    ((header >> 2) & 0xFF) as u8
}

// Pseudocode: Extract arity/size
fn header_arity(header: u64) -> u64 {
    header >> 10
}
```

---

## Heap Object Layouts

### Tuple

Fixed-size indexed collection:

```
TUPLE LAYOUT
════════════════════════════════════════════════════════════════════════════════

┌────────────────────────────────────────────────────────────────────────┐
│ HEADER: arity=N, tag=TUPLE                                    (8B)     │
├────────────────────────────────────────────────────────────────────────┤
│ element[0]: Term                                              (8B)     │
│ element[1]: Term                                              (8B)     │
│ ...                                                                    │
│ element[N-1]: Term                                            (8B)     │
└────────────────────────────────────────────────────────────────────────┘

Total size: 8 + N × 8 bytes

Example: [1 2 3]
  Header: arity=3, tag=TUPLE
  element[0] = SmallInt(1)
  element[1] = SmallInt(2)
  element[2] = SmallInt(3)
  Total: 32 bytes
```

### Vector

Persistent vector with structural sharing:

```
VECTOR LAYOUT
════════════════════════════════════════════════════════════════════════════════

┌────────────────────────────────────────────────────────────────────────┐
│ HEADER: arity=capacity, tag=VECTOR                            (8B)     │
├────────────────────────────────────────────────────────────────────────┤
│ length: u64                                                   (8B)     │
├────────────────────────────────────────────────────────────────────────┤
│ element[0]: Term                                              (8B)     │
│ element[1]: Term                                              (8B)     │
│ ...                                                                    │
│ element[capacity-1]: Term                                     (8B)     │
└────────────────────────────────────────────────────────────────────────┘

Total size: 16 + capacity × 8 bytes

length ≤ capacity (capacity allows room for growth without reallocation)
```

### String

UTF-8 encoded text:

```
STRING LAYOUT
════════════════════════════════════════════════════════════════════════════════

┌────────────────────────────────────────────────────────────────────────┐
│ HEADER: arity=byte_length, tag=STRING                         (8B)     │
├────────────────────────────────────────────────────────────────────────┤
│ UTF-8 bytes (byte_length bytes, padded to 8-byte alignment)            │
└────────────────────────────────────────────────────────────────────────┘

Total size: 8 + align8(byte_length) bytes

Note: Empty strings have arity=0, so size is 8 bytes (header only).
```

### Map

Association structure (implementation may vary):

```
MAP LAYOUT (Association List)
════════════════════════════════════════════════════════════════════════════════

┌────────────────────────────────────────────────────────────────────────┐
│ HEADER: arity=entry_count, tag=MAP                            (8B)     │
├────────────────────────────────────────────────────────────────────────┤
│ entries: Term (pointer to pair chain or nil)             (8B)     │
└────────────────────────────────────────────────────────────────────────┘

Total size: 16 bytes (entries stored as linked pairs)

Future: May switch to HAMT for large maps
```

### Compiled Function

Bytecode with constants:

**Note**: Unlike other objects where `arity` has type-specific meaning, FUN stores
the **total object size in words** in the header's arity field. This allows GC to
determine object size from the header alone without chasing pointers (critical
since the object may have a forwarding pointer during GC).

```
COMPILED FUNCTION LAYOUT
════════════════════════════════════════════════════════════════════════════════

┌────────────────────────────────────────────────────────────────────────┐
│ HEADER: arity=total_words, tag=FUN (size = arity × 8 bytes)   (8B)     │
├────────────────────────────────────────────────────────────────────────┤
│ fn_arity: u8 | variadic: u8 | locals: u8 | _pad: u8           (4B)     │
│ code_len: u16 | const_count: u16                              (4B)     │
├────────────────────────────────────────────────────────────────────────┤
│ bytecode: [u8; code_len] (padded to 8-byte alignment)                  │
├────────────────────────────────────────────────────────────────────────┤
│ constant[0]: Term                                             (8B)     │
│ constant[1]: Term                                             (8B)     │
│ ...                                                                    │
│ constant[const_count-1]: Term                                 (8B)     │
└────────────────────────────────────────────────────────────────────────┘
```

### Closure

Function with captured environment:

```
CLOSURE LAYOUT
════════════════════════════════════════════════════════════════════════════════

┌────────────────────────────────────────────────────────────────────────┐
│ HEADER: arity=capture_count, tag=CLOSURE                      (8B)     │
├────────────────────────────────────────────────────────────────────────┤
│ function: Term (pointer to FUN)                               (8B)     │
├────────────────────────────────────────────────────────────────────────┤
│ capture[0]: Term                                              (8B)     │
│ capture[1]: Term                                              (8B)     │
│ ...                                                                    │
│ capture[capture_count-1]: Term                                (8B)     │
└────────────────────────────────────────────────────────────────────────┘

Total size: 16 + capture_count × 8 bytes
```

### Bignum

Arbitrary precision integer:

```
BIGNUM LAYOUT
════════════════════════════════════════════════════════════════════════════════

┌────────────────────────────────────────────────────────────────────────┐
│ HEADER: arity=limb_count, tag=BIGNUM                          (8B)     │
├────────────────────────────────────────────────────────────────────────┤
│ sign: u64 (0 = positive, 1 = negative)                        (8B)     │
├────────────────────────────────────────────────────────────────────────┤
│ limb[0]: u64 (least significant)                              (8B)     │
│ limb[1]: u64                                                  (8B)     │
│ ...                                                                    │
│ limb[limb_count-1]: u64 (most significant)                    (8B)     │
└────────────────────────────────────────────────────────────────────────┘

Total size: 16 + limb_count × 8 bytes
```

### Float

64-bit IEEE 754:

```
FLOAT LAYOUT
════════════════════════════════════════════════════════════════════════════════

┌────────────────────────────────────────────────────────────────────────┐
│ HEADER: arity=1, tag=FLOAT                                    (8B)     │
├────────────────────────────────────────────────────────────────────────┤
│ value: f64 (IEEE 754 double)                                  (8B)     │
└────────────────────────────────────────────────────────────────────────┘

Total size: 16 bytes
```

### ProcBin (Large Binary Reference)

Reference to off-heap binary:

```
PROCBIN LAYOUT
════════════════════════════════════════════════════════════════════════════════

┌────────────────────────────────────────────────────────────────────────┐
│ HEADER: arity=0, tag=PROCBIN                                  (8B)     │
├────────────────────────────────────────────────────────────────────────┤
│ binary_addr: Vaddr (pointer to RefcBinary in realm binary heap) (8B)   │
├────────────────────────────────────────────────────────────────────────┤
│ offset: u32 | size: u32                                       (8B)     │
└────────────────────────────────────────────────────────────────────────┘

Total size: 24 bytes

Points directly to RefcBinary in realm's binary heap:
  - RefcBinary { refcount: AtomicU32, size: u32, data: [u8; size] }
  - Reference counted (shared between processes within same realm)
  - Immutable after creation
```

### SubBin (Binary View)

View into an existing binary (for slicing without copy):

```
SUBBIN LAYOUT
════════════════════════════════════════════════════════════════════════════════

┌────────────────────────────────────────────────────────────────────────┐
│ HEADER: arity=0, tag=SUBBIN                                   (8B)     │
├────────────────────────────────────────────────────────────────────────┤
│ original: Term (pointer to ProcBin or HeapString)             (8B)     │
├────────────────────────────────────────────────────────────────────────┤
│ offset: u32 | size: u32                                       (8B)     │
└────────────────────────────────────────────────────────────────────────┘

Total size: 24 bytes

SubBin enables zero-copy binary slicing:
  - original: The backing binary (increments refcount if RefcBinary)
  - offset: Starting byte offset into original
  - size: Number of bytes in this view
```

### PID (Process Identifier)

Heap-allocated process identifier:

```
PID LAYOUT
════════════════════════════════════════════════════════════════════════════════

┌────────────────────────────────────────────────────────────────────────┐
│ HEADER: arity=0, tag=PID                                      (8B)     │
├────────────────────────────────────────────────────────────────────────┤
│ index: u32 | generation: u32                                  (8B)     │
└────────────────────────────────────────────────────────────────────────┘

Total size: 16 bytes

Fields:
  - index: Slot index in process table
  - generation: Incremented on slot reuse (ABA safety)
```

### REF (Unique Reference)

Heap-allocated unique reference (for monitors, etc.):

```
REF LAYOUT
════════════════════════════════════════════════════════════════════════════════

┌────────────────────────────────────────────────────────────────────────┐
│ HEADER: arity=0, tag=REF                                      (8B)     │
├────────────────────────────────────────────────────────────────────────┤
│ id_high: u32 | id_low: u32                                    (8B)     │
└────────────────────────────────────────────────────────────────────────┘

Total size: 16 bytes

Fields:
  - id_high/id_low: 64-bit unique identifier (process-local counter + pid hash)

References are globally unique and never reused.
```

---

## Forwarding Pointers (GC)

During garbage collection, copied objects leave a **forwarding pointer** at their old location. This uses the special header tag `0xFF`:

```
FORWARDING POINTER
════════════════════════════════════════════════════════════════════════════════

Before GC (original object):
┌────────────────────────────────────────────────────────────────────────┐
│ HEADER: arity=N, tag=TUPLE                                             │
├────────────────────────────────────────────────────────────────────────┤
│ data...                                                                │
└────────────────────────────────────────────────────────────────────────┘

After copying (old location becomes forwarding pointer):
┌────────────────────────────────────────────────────────────────────────┐
│ HEADER: new_address, tag=FORWARD (0xFF)                                │
└────────────────────────────────────────────────────────────────────────┘

The new_address is stored in the arity field (54 bits), which is sufficient
for any heap address on current 64-bit systems (typically 48-bit virtual).

Detection:
  fn is_forwarding(header: u64) -> bool {
      header_object_tag(header) == 0xFF
  }

  fn forward_address(header: u64) -> u64 {
      (header >> 10) << 3  // Extract and re-align address
  }
```

**Note**: Pairs (tag `01`) don't have headers. During GC, a forwarded pair is detected by checking if the HEAD position contains a forwarding marker. The GC uses a special encoding: if HEAD has primary tag `00` (which is impossible for a valid term since `00` = HEADER, and headers only appear at object start), it's a forwarding pointer.

---

## Object Sizes and Alignment

All heap allocations are **8-byte aligned**. Object sizes vary by type:

```
OBJECT SIZES
════════════════════════════════════════════════════════════════════════════════

Pair (no header):          16 bytes (head + rest)
Empty string:               8 bytes (header only, arity=0)
Float:                     16 bytes (header + f64)
Map:                       16 bytes (header + entries term)
Tuple(0):                   8 bytes (header only)
Tuple(N):              8 + N×8 bytes
String(len):       8 + align8(len) bytes
Closure(N):           16 + N×8 bytes

MINIMUM SIZES for GC forwarding:
- Boxed objects: 8 bytes minimum (header becomes forwarding header)
- Pairs: 16 bytes (head=marker, rest=new address) - always satisfied
```

**Forwarding pointers** during GC do NOT require extra space:
- **Boxed objects**: The 8-byte header is replaced with an 8-byte forwarding header
- **Pairs**: The 16-byte pair uses head as marker and rest as new address

```
// Pseudocode: Calculate allocation size
fn alloc_size(requested: usize) -> usize {
    align8(requested)  // Just align to 8 bytes
}
```

---

## Tag Summary

```
COMPLETE TAG HIERARCHY
════════════════════════════════════════════════════════════════════════════════

Primary (bits 0-1):
├── 00 HEADER (heap object marker)
│   └── Object tag (bits 2-9):
│       ├── 0x00 TUPLE
│       ├── 0x01 VECTOR
│       ├── 0x02 MAP
│       ├── 0x03 STRING
│       ├── 0x04 BINARY
│       ├── 0x05 BIGNUM
│       ├── 0x06 FLOAT
│       ├── 0x07 FUN
│       ├── 0x08 CLOSURE
│       ├── 0x09 PID
│       ├── 0x0A REF
│       ├── 0x0B PROCBIN
│       └── 0xFF FORWARD (GC)
│
├── 01 LIST (pair pointer)
│
├── 10 BOXED (heap object pointer)
│
└── 11 IMMEDIATE
    └── Subtag (bits 2-3):
        ├── 00 SMALL_INT (60-bit signed)
        ├── 01 SYMBOL (interned index)
        ├── 10 KEYWORD (interned index)
        └── 11 SPECIAL
            └── Tertiary (bits 4-7):
                ├── 0000 NIL
                ├── 0001 TRUE
                ├── 0010 FALSE
                └── 0011 UNBOUND
```

---

## Implementation Notes

### Alignment

All heap allocations are 8-byte aligned. This is mandatory for:
- Tagged pointer low bits to be zero
- Efficient word access on 64-bit systems
- Correct behavior on architectures requiring aligned access

### Endianness

The layout assumes little-endian byte order (x86_64, aarch64). Tag bits are in the low addresses.

### Platform Specifics

- **x86_64**: Uses 48-bit virtual addresses; 54-bit arity field is sufficient
- **aarch64**: Same as x86_64 for user space

### Register Conventions

- X registers hold Terms (tagged words)
- Y registers (stack) hold Terms
- All registers are 8 bytes

---

## References

- [BEAM Wisdoms - Memory Layout](https://github.com/kvakvs/beam-wisdoms/blob/master/docs/source/indepth-memory-layout.rst)
- [The BEAM Book](https://blog.stenmans.org/theBeamBook/)
- [Garbage Collection](garbage-collection.md) - How GC uses headers and forwarding pointers
- [Process Model](process-model.md) - Heap and stack layout within processes
