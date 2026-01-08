# Realm Memory Layout

This document details the complete virtual address space layout of a realm, including shared code, inherited regions, process memory, and device mappings.

## VSpace Overview

A realm's VSpace contains several distinct regions:

```
REALM VSPACE (64-bit address space)
════════════════════════════════════════════════════════════════════════

High addresses:
┌─────────────────────────────────────────────────────────────────────┐
│  Kernel reserved (unmapped in userspace)                            │
├─────────────────────────────────────────────────────────────────────┤
│  MMIO / Device region (if driver realm)                             │
├─────────────────────────────────────────────────────────────────────┤
│  Process region (process heaps)                                     │
├─────────────────────────────────────────────────────────────────────┤
│  Realm-local data (RW)                                              │
├─────────────────────────────────────────────────────────────────────┤
│  Inherited regions (RO, from parent chain)                          │
├─────────────────────────────────────────────────────────────────────┤
│  Shared code (RX/R, same physical frames)                           │
├─────────────────────────────────────────────────────────────────────┤
│  Worker stacks (one per TCB)                                        │
├─────────────────────────────────────────────────────────────────────┤
│  Guard pages (NULL protection)                                      │
└─────────────────────────────────────────────────────────────────────┘
Low addresses (0x0)
```

---

## Region Details

### 1. Shared Code Region

Contains code and data shared across ALL realms (same physical frames):

```
SHARED CODE REGION
────────────────────────────────────────────────────────────────────────

Lona VM code (RX):
┌─────────────────────────────────────────────────────────────────────┐
│  .text (executable code)                                            │
│  .rodata (constants, jump tables)                                   │
│                                                                     │
│  Physical frames: SHARED across all realms                          │
│  Permissions: Read + Execute                                        │
│  Size: ~1-2 MB                                                      │
└─────────────────────────────────────────────────────────────────────┘

Core library (R):
┌─────────────────────────────────────────────────────────────────────┐
│  Compiled lona.core + metadata                                      │
│  Compiled lona.process + metadata                                   │
│  Other core namespaces                                              │
│                                                                     │
│  Physical frames: SHARED across all realms                          │
│  Permissions: Read only                                             │
│  Size: ~1-5 MB                                                      │
│                                                                     │
│  Note: Source is embedded in VM binary and compiled at boot.        │
└─────────────────────────────────────────────────────────────────────┘
```

**Key point**: Same physical memory mapped read-only into every realm. Memory efficient and cache friendly.

---

### 2. Inherited Regions

Each realm can inherit code and data from its ancestors. These are mapped read-only with **fixed virtual addresses** so pointers remain valid across realm boundaries.

```
INHERITED REGIONS
────────────────────────────────────────────────────────────────────────

For a realm at depth N (has N ancestors):

┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  Ancestor 0 (root realm):                                           │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  Code sub-region:                                           │    │
│  │  - Bytecode segments                                        │    │
│  │  - Var bindings (symbol → value pointer)                    │    │
│  │  - Var metadata (docstrings, arglists)                      │    │
│  │  - Interned symbols/keywords                                │    │
│  │  - Small constants (<64 bytes)                              │    │
│  │                                                             │    │
│  │  Binary sub-region:                                         │    │
│  │  - Large binaries (≥64 bytes)                               │    │
│  │  - Referenced by vars in code region                        │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                     │
│  Ancestor 1 (init realm):                                           │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  (Same structure: code + binary sub-regions)                │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                     │
│  ... up to ancestor N-1 (direct parent) ...                         │
│                                                                     │
│  All mapped READ-ONLY from parent's memory                          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

#### Why Two Sub-Regions Per Ancestor

Vars can reference large binaries. The code region contains structured data (bytecode, var bindings), while the binary region contains large unstructured data:

```
Parent Realm's Memory:
┌─────────────────────────────────────────────────────────────────────┐
│  Code Region                    Binary Region                       │
│  ┌───────────────────┐          ┌───────────────────┐               │
│  │ Var: image-data   │          │                   │               │
│  │ Ptr: ─────────────┼─────────▶│ [10 MB binary]    │               │
│  │                   │          │                   │               │
│  │ Var: config       │          │                   │               │
│  │ Ptr: ─────────────┼─────────▶│ [500 KB json]     │               │
│  └───────────────────┘          └───────────────────┘               │
└─────────────────────────────────────────────────────────────────────┘

Child maps BOTH regions at SAME virtual addresses.
Pointers in code region still work because addresses are identical.
```

#### Fixed Virtual Addresses

```
WHY FIXED ADDRESSES WORK
────────────────────────────────────────────────────────────────────────

Parent realm stores at 0x0004_4000_0000:
  (def x {:name "hello" :data <10MB binary>})

  Map @ 0x0004_4001_0000 contains:
    :name → 0x0004_4001_0050 (string in code region)
    :data → 0x0005_0010_0000 (binary in binary region)

Child maps parent's regions at SAME virtual addresses:
  Parent's code region   → 0x0004_4000_0000 (RO)
  Parent's binary region → 0x0005_0000_0000 (RO)

Child looks up 'x':
  1. Reads binding → pointer to 0x0004_4001_0000
  2. Reads map → pointers still valid!
  3. Follows :data → reads binary at 0x0005_0010_0000

ALL POINTERS WORK because virtual addresses match.
```

#### Sizing

Illustrative sizing (exact values TBD):

| Component | Virtual Size | Notes |
|-----------|--------------|-------|
| Code sub-region per ancestor | 1 GB | Bytecode, vars, symbols |
| Binary sub-region per ancestor | 4 GB | Large binaries |
| Total per ancestor | 5 GB | |
| 10 ancestors | 50 GB virtual | Trivial in 64-bit space |

Physical usage is only what's actually allocated - pages mapped on demand.

---

### 3. Realm-Local Data Region

This realm's own data (read-write):

```
REALM-LOCAL DATA REGION
────────────────────────────────────────────────────────────────────────

Scheduler State:
┌─────────────────────────────────────────────────────────────────────┐
│  Run queue (priority levels)                                        │
│  Wait queues (processes blocked on receive)                         │
│  Timer heap (sleeping processes)                                    │
│  Scheduler statistics                                               │
└─────────────────────────────────────────────────────────────────────┘

Process Table:
┌─────────────────────────────────────────────────────────────────────┐
│  Process descriptors array:                                         │
│    - PID → process metadata mapping                                 │
│    - State (running, waiting, exited)                               │
│    - Pointers to heap block, mailbox                                │
│    - Link/monitor relationships                                     │
│    - Reduction counter                                              │
└─────────────────────────────────────────────────────────────────────┘

Local Namespaces (this realm's definitions):
┌─────────────────────────────────────────────────────────────────────┐
│  Code sub-region (same structure as inherited):                     │
│    - Namespace registry                                             │
│    - Local bytecode                                                 │
│    - Var bindings                                                   │
│    - Interned symbols                                               │
│                                                                     │
│  Binary sub-region:                                                 │
│    - Large binaries                                                 │
│    - Reference counted within realm                                 │
└─────────────────────────────────────────────────────────────────────┘

Atom Table:
┌─────────────────────────────────────────────────────────────────────┐
│  Interned atoms/symbols                                             │
│  String → atom ID mapping                                           │
│  Shared across all processes in realm                               │
└─────────────────────────────────────────────────────────────────────┘

Port/Reference Registry:
┌─────────────────────────────────────────────────────────────────────┐
│  Active ports (external I/O)                                        │
│  Reference counter (for make-ref)                                   │
│  Monitor registry                                                   │
└─────────────────────────────────────────────────────────────────────┘
```

---

### 4. Process Region

Memory for Lonala processes, allocated from per-worker allocators.

#### Per-Worker Allocator Instances

Following BEAM's proven model, each worker (scheduler thread) has its own allocator instance for lock-free allocation:

```
PER-WORKER ALLOCATION (Lock-Free)
────────────────────────────────────────────────────────────────────────

                    PROCESS POOL REGION
                           │
       ┌───────────────────┼───────────────────┐
       ▼                   ▼                   ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Worker 0   │     │  Worker 1   │     │  Worker 2   │
│  Allocator  │     │  Allocator  │     │  Allocator  │
│             │     │             │     │             │
│ ┌─────────┐ │     │ ┌─────────┐ │     │ ┌─────────┐ │
│ │Carrier 0│ │     │ │Carrier 0│ │     │ │Carrier 0│ │
│ ├─────────┤ │     │ ├─────────┤ │     │ ├─────────┤ │
│ │Carrier 1│ │     │ │Carrier 1│ │     │ │Carrier 1│ │
│ └─────────┘ │     │ └─────────┘ │     │ └─────────┘ │
│  LOCK-FREE  │     │  LOCK-FREE  │     │  LOCK-FREE  │
└──────┬──────┘     └──────┬──────┘     └──────┬──────┘
       │                   │                   │
       ▼                   ▼                   ▼
  Processes on        Processes on       Processes on
   Worker 0            Worker 1           Worker 2

Processes running on a worker allocate from that worker's instance.
No coordination needed for the common case (allocation).
```

#### Two-Heap Architecture

Each process owns **two memory blocks** following the BEAM model. For the complete memory model, see [Process Model](process-model.md).

```
PROCESS MEMORY LAYOUT (Two Heaps)
────────────────────────────────────────────────────────────────────────

Young Heap (stack + young objects):
    ┌────────────────────────────────────────────────────────────────┐
    │   STACK                      FREE                 YOUNG HEAP   │
    │   (grows down)              SPACE                 (grows up)   │
    │                                                                │
    │   [frame1][frame0]◄─stop           htop─►[tuple][cons][string] │
    │                                                                │
    └────────────────────────────────────────────────────────────────┘
    Out of memory when htop >= stop → triggers Minor GC

Old Heap (promoted objects):
    ┌────────────────────────────────────────────────────────────────┐
    │   [promoted][promoted][promoted]           │       FREE        │
    │   (survived Minor GC)                      │◄─ old_htop        │
    └────────────────────────────────────────────────────────────────┘
    Collected only during Major GC (fullsweep)
```

#### Generational GC

```
GENERATIONAL GARBAGE COLLECTION
────────────────────────────────────────────────────────────────────────

Initial young heap: ~2 KB (enables millions of tiny processes)

Minor GC (when young heap full):
  1. Scan roots (stack, registers)
  2. Copy live young objects to OLD heap (promotion)
  3. Reset young heap (all space now free)
  4. Free heap fragments

Major GC (fullsweep, less frequent):
  1. Collect BOTH young and old heaps
  2. Compact all live data into new young heap
  3. Allocate fresh empty old heap

Key properties:
- Minor GC is fast (~10-100 µs) - only touches young heap
- Major GC is slower but reclaims old generation garbage
- Per-process GC (no global pauses)
```

#### Process Table Entry

```
PROCESS TABLE ENTRY
────────────────────────────────────────────────────────────────────────

┌─────────────────────────────────────────────────────────────────────┐
│  PID: 42                                                            │
│  State: running                                                     │
│                                                                     │
│  Young Heap:                                                        │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  heap:  0x0000_0010_0400_0000  (base address)               │    │
│  │  hend:  0x0000_0010_0400_1000  (end address)                │    │
│  │  htop:  0x0000_0010_0400_0800  (young heap top, grows up)   │    │
│  │  stop:  0x0000_0010_0400_0F00  (stack ptr, grows down)      │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                     │
│  Old Heap:                                                          │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  old_heap: 0x0000_0010_0401_0000 (base address)             │    │
│  │  old_hend: 0x0000_0010_0401_2000 (end address)              │    │
│  │  old_htop: 0x0000_0010_0401_0400 (old heap top)             │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                     │
│  mbuf_list: (heap fragments from message passing)                   │
│  Mailbox: MPSC queue (head/tail pointers)                           │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

Young heap contains: stack frames, recently allocated objects
Old heap contains: promoted objects (survived Minor GC)
Both contain: cons cells, tuples, vectors, maps, closures,
              small binaries (<64 bytes), refs to large binaries
```

#### Large Binary Handling

Binaries ≥64 bytes are stored in a realm-wide binary heap with reference counting:

```
LARGE BINARY HANDLING
────────────────────────────────────────────────────────────────────────

Process heaps contain:          Binary heap (realm-wide):
┌─────────┐                     ┌─────────────────────────────┐
│ ProcRef ├────────────────────▶│ 10 MB image data            │
└─────────┘                     │ refcount: 3                 │
┌─────────┐                     └─────────────────────────────┘
│ ProcRef ├────────────────────▶│ 500 KB JSON payload         │
└─────────┘         ┌──────────▶│ refcount: 2                 │
┌─────────┐         │           └─────────────────────────────┘
│ ProcRef ├─────────┘
└─────────┘

Small binaries (<64 bytes): copied into process heap
Large binaries (≥64 bytes): reference counted, shared

Benefits:
- Zero-copy message passing for large binaries
- Process heaps stay small
- Per-process GC stays fast
```

---

### 5. Worker Stacks Region

Native stacks for Lona VM workers (TCBs):

```
WORKER STACKS REGION
────────────────────────────────────────────────────────────────────────

Worker 0 (TCB 0):
┌─────────────────────────────────────────────────────────────────────┐
│  IPC buffer (4 KB, required by seL4)                                │
│  Guard page                                                         │
│  Native stack (~256 KB)                                             │
│    - Used by Lona VM Rust code                                      │
│    - Interpreter call frames                                        │
│    - NOT Lonala process stacks (those are in process region)        │
│  Guard page                                                         │
└─────────────────────────────────────────────────────────────────────┘

Worker 1 (TCB 1):
┌─────────────────────────────────────────────────────────────────────┐
│  (Same structure)                                                   │
└─────────────────────────────────────────────────────────────────────┘

One per worker/TCB in the realm.
```

---

### 6. MMIO / Device Region

Only mapped in driver realms that have device capabilities:

```
MMIO / DEVICE REGION (Driver Realms Only)
────────────────────────────────────────────────────────────────────────

UART Device (example):
┌─────────────────────────────────────────────────────────────────────┐
│  UART registers (4 KB)                                              │
│    - Physical: 0x0900_0000 (hardware address)                       │
│    - Mapped via Device Untyped capability                           │
│    - Permissions: RW, uncached                                      │
└─────────────────────────────────────────────────────────────────────┘

NIC Device (example):
┌─────────────────────────────────────────────────────────────────────┐
│  NIC registers (64 KB)                                              │
│  DMA ring buffers (1 MB)                                            │
│    - Must be contiguous physical memory                             │
│    - Device reads/writes directly                                   │
└─────────────────────────────────────────────────────────────────────┘

Framebuffer (example):
┌─────────────────────────────────────────────────────────────────────┐
│  GPU framebuffer (16 MB)                                            │
│    - Write-combining memory for performance                         │
└─────────────────────────────────────────────────────────────────────┘

Interrupt handling via seL4 Notification capabilities
(not memory-mapped, but realm receives notifications)
```

---

## Complete Layout Summary

Illustrative virtual address assignment for a realm at depth 10:

```
Address Range                    Region                    Permissions
─────────────────────────────────────────────────────────────────────────
0xFFFF_8000_0000_0000+          Kernel                    (none)
0x0000_0100_0000_0000+          (Reserved/Future)         (unmapped)
0x0000_00F0_0000_0000           MMIO/Devices              RW uncached
0x0000_0010_0000_0000           Process heaps             RW
0x0000_0009_0000_0000           Realm binary heap         RW
0x0000_0008_0000_0000           Realm-local code/data     RW
0x0000_0004_0000_0000           Inherited (ancestors)     RO
0x0000_0001_0000_0000           Shared code               RX/RO
0x0000_0000_1000_0000           Worker stacks             RW
0x0000_0000_0000_1000           Guard page                (unmapped)
0x0000_0000_0000_0000           NULL guard                (unmapped)
```

**Note**: Specific addresses are illustrative. Exact values will be determined during implementation.

---

## Vars and Code Compilation

This section explains what vars, functions, and closures actually *are* - the data structures that live in the code regions.

### Var Structure

A **Var** is a named, mutable binding in a namespace. It provides indirection that enables live code updates.

Vars use a two-level structure for atomic updates (MVCC-style):

```
VAR STRUCTURE
════════════════════════════════════════════════════════════════════════

struct VarSlot {
    content: AtomicPtr<VarContent>,  // Acquire on load, Release on store
    // NOTE: VarSlot's own Vaddr is used as VarId for process-bound lookup
    // No separate id field needed - the address IS the unique identifier
}

struct VarContent {
    name: Vaddr,           // Interned symbol
    namespace: Vaddr,      // Containing namespace
    root: Value,           // Inline 16-byte tagged value (NOT a pointer)
    flags: u32,            // PROCESS_BOUND | NATIVE | MACRO | PRIVATE
}

// VarId is simply the VarSlot's address (stable in realm code region)
type VarId = Vaddr;

SIZE: VarSlot ~8 bytes, VarContent ~40 bytes
LOCATION: Code region (owned by defining realm)
```

**Design decisions**:
- `root` is an inline `Value` (not a pointer) so vars can hold any Lonala value directly
- Metadata is stored in a separate realm metadata table, not inline in VarContent
- VarSlots have stable addresses for bytecode pointers; VarContent is replaced atomically on update
- VarSlot address serves as VarId for process-bound lookups (no separate id field needed)

**Key insight**: Vars are *indirect* references. Code calls functions through vars, not direct pointers. When a var is rebound, all callers see the new value automatically.

### CompiledFn Structure

A **CompiledFn** is the result of compiling a `fn*` form. It represents a pure function without captures.

```
COMPILEDFN STRUCTURE
════════════════════════════════════════════════════════════════════════

struct CompiledFn {
    bytecode: Vaddr,            // Pointer to bytecode array
    bytecode_len: u32,          // Length in bytes
    constants: Vaddr,           // Pointer to constant pool (array of Values)
    constants_len: u16,         // Number of constants
    arity: u8,                  // Required parameters
    variadic: bool,             // Accepts &rest?
    num_locals: u8,             // Y registers needed
    params: Vaddr,              // Parameter names tuple [x y z]
    source_form: Vaddr,         // Original (fn* ...) form (for debugging/macros)
    source_file: Vaddr,         // Source file path string (or nil)
    source_line: u32,           // Line number in source file
}

SIZE: ~56 bytes
LOCATION: Initially on process heap; copied to code region by `def`
```

**Value type**: `Value::CompiledFn(Vaddr)` - separate from Closure

### Closure Structure

A **Closure** pairs a function with captured values:

```
CLOSURE STRUCTURE
════════════════════════════════════════════════════════════════════════

struct Closure {
    function: Vaddr,            // → CompiledFn
    captures: Vaddr,            // → Tuple of captured values
}

SIZE: ~16 bytes
LOCATION: Initially on process heap; copied to code region by `def`

Example:
  (def add-5 ((fn* [x] (fn* [y] (+ x y))) 5))

  add-5 is a Closure:
    function → CompiledFn for (fn* [y] (+ x y))
    captures → [5]
```

**Value type**: `Value::Closure(Vaddr)` - separate from CompiledFn

### The `def` Flow: Process Heap → Realm Storage

When you evaluate `(def name value)`, the value moves from process heap to persistent realm storage:

```
DEF FLOW
════════════════════════════════════════════════════════════════════════

(def square (fn* [x] (* x x)))

1. READER: Produces AST on process heap
   └── List: (def square (fn* [x] (* x x)))

2. EVALUATOR: Recognizes def special form
   └── Extracts name: 'square
   └── Evaluates value expr: (fn* [x] (* x x))

3. fn* COMPILATION: Produces CompiledFn on PROCESS HEAP
   └── Compiles body to bytecode
   └── Allocates CompiledFn struct on process heap
   └── Returns Value::Function(process_heap_addr)

4. def DECIDES LOCATION:
   If regular var (not process-bound):
     └── Deep copy CompiledFn to REALM code region
     └── Copy bytecode, constants, source form
     └── Create/update VarContent with root = Value::Function(realm_addr)
   If process-bound var:
     └── Store Value::Function in process.bindings
     └── CompiledFn stays on process heap

5. VAR CREATION/UPDATE: In REALM code region
   └── VarSlot atomically points to new VarContent
   └── Updates namespace binding table

6. CLEANUP: Process heap AST becomes garbage
   └── VarSlot and VarContent persist in realm storage
   └── For regular vars, CompiledFn is now in realm (process copy is garbage)
```

**Why compile to process heap first?**
- Anonymous functions `(spawn (fn* [] ...))` stay in process, never bound to vars
- Process-bound vars `(def ^:process-bound *handler* (fn* ...))` keep functions in process
- Only `def` to regular vars triggers deep copy to realm
- This uniform flow simplifies the compiler

### Late Binding Semantics

Var indirection enables live code updates:

```
LATE BINDING
════════════════════════════════════════════════════════════════════════

;; Original
(def process-request (fn* [req] (handle req)))

;; Code that uses it
(defn handler [req] (process-request req))

;; Later, rebind:
(def process-request (fn* [req]
  (log "Processing:" req)
  (handle req)))

;; handler NOW calls NEW version - no restart needed!

The bytecode does LOOKUP_VAR at runtime.
Rebinding changes var.root pointer.
All callers automatically see new value.
```

### Source Preservation

Lonala preserves source forms for introspection:

```
SOURCE PRESERVATION
════════════════════════════════════════════════════════════════════════

(source square)      → (fn* [x] (* x x))
(meta #'square)      → {:arglists ([x]) :file "..." :line 1}
(closed-overs add-5) → {x 5}

Enables: REPL inspection, debugging, live documentation
```

---

## Value Storage Rules

Where different types of values are stored:

| Value Type | Storage Location | Owner Access | Child Access |
|------------|------------------|--------------|--------------|
| Bytecode | Code region | RX | RX (shared frames) |
| Var metadata | Code region | RW | RO (shared frames) |
| Interned symbols | Code region | RW | RO (shared frames) |
| Interned keywords | Code region | RW | RO (shared frames) |
| Small literals (<64B) | Code region | RO | RO (shared frames) |
| Var bindings (pointers) | Code region | RW | RO (shared frames) |
| Large binary content | Binary region | RO | RO (shared frames) |
| Binary refcounts | Realm-local table | RW | N/A (per-realm) |
| Process heap values | Process heap | RW | N/A (per-process) |
| Mailbox messages | Process mailbox | RW | N/A (copied on send) |

**Note on mutability:**
- Var bindings are RW for the owning realm but RO for children (same physical frames, different permissions)
- Binary content is immutable; refcounts are stored in a separate mutable table per realm
- When owner updates a var binding, children see the change immediately (live sharing)

---

## Memory Allocation Flow

How memory allocation works at runtime:

```
Process needs memory (e.g., (cons 1 2)):
────────────────────────────────────────────────────────────────────────

1. Runtime checks process heap
   └── Has space? → Bump allocate, done
   └── No space? → Try GC

2. GC runs on process heap
   └── Freed enough? → Continue
   └── Need more? → Grow heap

3. Runtime grows process heap
   └── Request larger heap from worker's allocator
   └── Within process limit? → Allocate and copy live data
   └── At limit? → OOM for this process

4. Realm pool needs more pages
   └── Page fault → Lona Memory Manager handles
   └── Within realm budget? → Lona Memory Manager maps pages
   └── At budget? → Realm OOM (runtime decides policy)
```

---

## Inherited Region Management

### Live Sharing Semantics

Inherited regions are **live-shared**, not snapshots:

```
LIVE SHARING
════════════════════════════════════════════════════════════════════════

Parent realm:                     Child realm:
┌─────────────────────┐           ┌─────────────────────┐
│ Code region (RW)    │           │ Code region (RO)    │
│                     │           │                     │
│ (def x 42)          │ ────────▶ │ x = 42 (sees same)  │
│       │             │  same     │                     │
│       ▼             │  frames   │                     │
│ (def x 100)         │ ────────▶ │ x = 100 (sees new!) │
└─────────────────────┘           └─────────────────────┘

When parent updates var binding → child immediately sees new value
(same physical frames, parent has RW, child has RO mapping)
```

This enables:

- Hot code reloading (parent updates code, children see new version)
- Shared configuration updates
- Dynamic system evolution without realm restart

**Atomicity guarantee**: Var binding updates are atomic. A child reading a var always sees either the old value or the new value, never a partially-updated (torn) state.

### Var Shadowing

Children can shadow inherited vars with local definitions:

```
VAR SHADOWING
════════════════════════════════════════════════════════════════════════

Parent defines: (def config {:debug false})

Child A (uses inherited):
    config → {:debug false}  (from parent's code region)

Child B (shadows with local):
    (def config {:debug true})  ; written to child's local code region
    config → {:debug true}       (local takes precedence)

Parent updates: (def config {:debug false :verbose true})

Child A sees: {:debug false :verbose true}  (live update)
Child B sees: {:debug true}                  (still shadowed)
```

Var lookup order: local region → parent → grandparent → ... → core library

### Append-Only Model

For initial implementation, code/binary regions are **append-only**:

- New definitions append to regions
- Rebinding a var updates the binding pointer but old value remains in memory
- No garbage collection of code regions (known limitation)

This simplifies implementation. GC for these regions is a future concern.

### Future GC Considerations

When parent rebinds a var, old value becomes unreachable. Options for future:

1. **Mark dead space**: Track unreachable bytes, compact when creating new child
2. **Generational scheme**: Young/old generations within append-only regions
3. **Snapshot on fork**: Give child a compacted snapshot, parent continues with fragmentation

These are deliberately left undefined for future refinement.

---

## Summary

| Region | Contents | Sharing | Growth |
|--------|----------|---------|--------|
| **Shared code** | Lona VM, core lib | All realms (same frames) | Static |
| **Inherited** | Parent bytecode/vars/binaries | Parent→children (RO) | Append-only |
| **Realm-local** | Local vars, scheduler, tables | This realm only | Append-only |
| **Process** | Heaps, mailboxes | Per-process | Single block, grows via reallocation |
| **Worker stacks** | Native TCB stacks | Per-worker | Fixed |
| **MMIO** | Device registers, DMA | Driver realms only | Static |

This layout provides clear separation between shared, inherited, realm-local, and process-local data, while supporting hierarchical code inheritance and efficient process memory management.
