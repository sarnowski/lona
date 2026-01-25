# Garbage Collection

This document describes Lona's garbage collection system: a per-process generational copying collector inspired by BEAM, adapted for seL4's capability-based memory model.

> **Note**: All code examples in this document are **pseudocode** for illustration purposes.

## Overview

Lona uses a **per-process generational semi-space copying collector** following BEAM's proven design. Each Lonala process has its own isolated heap, enabling independent garbage collection with no global pauses.

### Key Properties

| Property | Description |
|----------|-------------|
| **Per-process** | Each process GCs independently; no stop-the-world pauses |
| **Generational** | Two generations: young (frequently collected) and old (rarely collected) |
| **Copying** | Live objects are copied to new space; dead objects are implicitly reclaimed |
| **Semi-space** | Young heap uses two spaces (from/to) for efficient copying |
| **Soft real-time** | GC pauses are per-process, typically 10-100 µs |
| **Immutable heap** | All heap objects are immutable after allocation (no write barriers needed) |

### Immutability Guarantee

**Critical invariant**: All heap objects in Lona are **immutable after allocation**. This is fundamental to the GC design.

```
WHY IMMUTABILITY MATTERS FOR GENERATIONAL GC
════════════════════════════════════════════════════════════════════════════════

Problem with mutable objects:
  - Old object A promoted to old heap
  - Later, A is mutated to point to young object B
  - Minor GC runs, only scanning young heap
  - A→B reference is NOT scanned (A is in old heap)
  - B appears dead, is not copied
  - A now has dangling pointer → CRASH

Solutions:
  1. Write barrier + remembered set (complex, runtime overhead)
  2. Immutable objects (no old→young references can be created)

Lona chooses #2: ALL HEAP OBJECTS ARE IMMUTABLE.

This is enforced by:
  - Lonala's functional semantics (persistent data structures)
  - No mutation primitives in the language
  - "Updates" create new objects via structural sharing
```

This immutability guarantee eliminates the need for write barriers or remembered sets, simplifying the GC and improving performance.

### Why Per-Process GC?

```
SYSTEM-WIDE VIEW
════════════════════════════════════════════════════════════════════════════════

Process A                  Process B                  Process C
    │                          │                          │
    │ GC in progress           │ executing                │ executing
    │ ████████████████         │                          │
    │ (A paused ~50 µs)        │ (unaffected)            │ (unaffected)
    │                          │                          │
    │ executing                │ GC in progress           │ executing
    │                          │ ████████████████         │
    │ (unaffected)             │ (B paused ~30 µs)       │ (unaffected)
    │                          │                          │
    ▼                          ▼                          ▼

- NO coordination between processes
- GC of Process A has ZERO impact on Process B or C
- System remains responsive during all GC operations
- This is fundamental to soft real-time properties
```

---

## BEAM's Garbage Collector (Reference)

Understanding BEAM's GC is essential since Lona's design closely follows it. This section documents BEAM's approach.

### BEAM Memory Layout

Each BEAM process has a private heap containing both stack and young objects:

```
BEAM PROCESS HEAP
════════════════════════════════════════════════════════════════════════════════

    ┌────────────────────────────────────────────────────────────────────────┐
    │                                                                        │
    │   STACK                         FREE                    YOUNG HEAP     │
    │   (grows down)                 SPACE                    (grows up)     │
    │                                                                        │
    │   ┌─────────┐                                          ┌─────────┐     │
    │   │ frame 2 │                                          │ tuple   │     │
    │   ├─────────┤                                          ├─────────┤     │
    │   │ frame 1 │                                          │ pair    │     │
    │   ├─────────┤                                          ├─────────┤     │
    │   │ frame 0 │◄─ SP                                HTOP─►│ binary  │     │
    │   └─────────┘                                          └─────────┘     │
    │        │                                                    │          │
    │        ▼                                                    ▲          │
    │   grows toward ──────────────────────────────────── grows toward       │
    │   lower addresses                               higher addresses       │
    │                                                                        │
    └────────────────────────────────────────────────────────────────────────┘

    GC triggered when: HTOP >= SP (heap meets stack)
```

### BEAM Generational Model

BEAM divides objects into generations using a **high-water mark**:

```
BEAM GENERATIONAL DIVISION
════════════════════════════════════════════════════════════════════════════════

BEFORE Minor GC:
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │ STACK │          │ OLD OBJECTS     │ YOUNG OBJECTS    │   (full)    │    │
│  │       │          │ (below h_water) │ (above h_water)  │             │    │
│  │ ───►  │          │    [old1][old2] │ [new1][new2][new3]│  ◄───      │    │
│  └───────┴──────────┴─────────────────┴──────────────────┴─────────────┘    │
│      ▲                      ▲                                   ▲           │
│     SP                high_water                              HTOP          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

The high_water mark separates:
- Objects below: survived previous GC (old generation)
- Objects above: recently allocated (young generation)

Minor GC only processes young objects (above high_water).
```

### BEAM's Two-Heap Architecture

BEAM actually uses **two separate memory blocks** for better performance:

```
BEAM TWO-HEAP ARCHITECTURE
════════════════════════════════════════════════════════════════════════════════

YOUNG HEAP (active allocation):
┌─────────────────────────────────────────────────────────────────────────────┐
│   STACK ◄─────────────────────────────────────────────────────► YOUNG HEAP  │
│   [frames]               FREE SPACE                    [recently allocated] │
└─────────────────────────────────────────────────────────────────────────────┘

OLD HEAP (promoted objects):
┌─────────────────────────────────────────────────────────────────────────────┐
│   PROMOTED OBJECTS                              │         FREE SPACE        │
│   [survived GC N-3][survived GC N-2][survived GC N-1]                       │
└─────────────────────────────────────────────────────────────────────────────┘

Minor GC: Copy live young objects → Old heap. Reset young heap.
Major GC: Collect both heaps → New young heap. Fresh old heap.
```

### BEAM Cheney's Copying Algorithm

BEAM uses Cheney's algorithm for efficient copying:

```
CHENEY'S COPYING ALGORITHM
════════════════════════════════════════════════════════════════════════════════

Phase 1: Root Scanning
────────────────────────────────────────────────────────────────────────────────
  Scan roots (stack, registers, process dictionary)
  For each pointer to young generation:
    → Copy object to old heap (or new young heap for major GC)
    → Leave "forwarding pointer" in original location
    → Update root to point to new location

Phase 2: Scan Copied Objects
────────────────────────────────────────────────────────────────────────────────
  Maintain a "scan" pointer in to-space
  While scan < allocation_pointer:
    For each pointer in current object:
      If points to from-space:
        If forwarding pointer exists → update to new location
        Else → copy object, leave forwarding pointer, update

FROM-SPACE (before):              TO-SPACE (after):
┌───────────────────────┐         ┌───────────────────────┐
│ [A]──────►[B]         │   ──►   │ [A']─────►[B']        │
│     [C]◄──┘           │   copy  │      [C']◄─┘          │
│ [D] (dead, no refs)   │         │                       │
└───────────────────────┘         └───────────────────────┘
                                  Dead object D not copied

Key properties:
- Single pass through live data (no separate mark phase)
- Compacts memory (no fragmentation)
- Work proportional to LIVE data, not total heap
```

### BEAM Heap Growth

BEAM uses a Fibonacci-like sequence for small heaps, then switches to percentage growth:

```
BEAM HEAP GROWTH SEQUENCE
════════════════════════════════════════════════════════════════════════════════

Phase 1: Fibonacci-like (small heaps)
────────────────────────────────────────────────────────────────────────────────
  233 → 377 → 610 → 987 → 1597 → 2584 → 4181 → 6765 → 10946 → ...  (words)

  In bytes (64-bit, 8 bytes/word):
  ~2KB → ~3KB → ~5KB → ~8KB → ~13KB → ~21KB → ~33KB → ~54KB → ~87KB → ...

Phase 2: 20% growth (large heaps, after ~1 megaword / ~8 MB)
────────────────────────────────────────────────────────────────────────────────
  8 MB → 9.6 MB → 11.5 MB → 13.8 MB → 16.6 MB → 19.9 MB → ...

Heap shrinking: When live data < 25% of heap capacity after GC
```

### BEAM Large Binary Handling

Large binaries (≥64 bytes) are stored outside the process heap:

```
BEAM LARGE BINARY HANDLING
════════════════════════════════════════════════════════════════════════════════

Process A heap:                    Shared Binary Heap:
┌─────────────────┐                ┌─────────────────────────────────────────┐
│ ProcBin ────────┼───────────────►│  refcount: 2                            │
│ (small header)  │                │  size: 10 MB                            │
└─────────────────┘                │  data: [................]               │
                                   └─────────────────────────────────────────┘
Process B heap:                                    ▲
┌─────────────────┐                                │
│ ProcBin ────────┼────────────────────────────────┘
│ (small header)  │
└─────────────────┘

- Only ProcBin (small pointer structure) in process heap
- Actual binary data in shared heap with reference counting
- Zero-copy message passing for large binaries
- MSO (Mark-Sweep Object) list tracks ProcBins for GC

Virtual Binary Heap:
- Tracks cumulative off-heap binary size per process
- Triggers early GC when binary memory becomes substantial
- Prevents binary memory from growing unbounded
```

---

## Lona's Garbage Collector

Lona adapts BEAM's GC model for seL4's capability-based memory system.

### Architectural Context

```
LONA MEMORY HIERARCHY
════════════════════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────────────────┐
│                           LONA MEMORY MANAGER                                │
│                                                                             │
│  - Manages physical memory (Untyped capabilities)                           │
│  - Allocates memory to realms via IPC                                       │
│  - Handles page faults for inherited regions                                │
│  - Enforces per-realm memory quotas                                         │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ allocates pages via IPC
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              REALM                                          │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │ Per-Worker Allocator Instances (lock-free within worker)            │    │
│  │                                                                     │    │
│  │  Worker 0 Allocator    Worker 1 Allocator    Worker N Allocator     │    │
│  │  ┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐    │    │
│  │  │ [carriers]      │   │ [carriers]      │   │ [carriers]      │    │    │
│  │  └────────┬────────┘   └────────┬────────┘   └────────┬────────┘    │    │
│  └───────────┼─────────────────────┼─────────────────────┼─────────────┘    │
│              │                     │                     │                  │
│              ▼                     ▼                     ▼                  │
│  ┌───────────────────┐ ┌───────────────────┐ ┌───────────────────┐          │
│  │ Process A         │ │ Process D         │ │ Process G         │          │
│  │ [young][old]      │ │ [young][old]      │ │ [young][old]      │          │
│  ├───────────────────┤ ├───────────────────┤ ├───────────────────┤          │
│  │ Process B         │ │ Process E         │ │ Process H         │          │
│  │ [young][old]      │ │ [young][old]      │ │ [young][old]      │          │
│  └───────────────────┘ └───────────────────┘ └───────────────────┘          │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │ Realm Binary Heap (reference-counted large binaries)                │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Process Memory Layout

Each process owns two memory blocks following BEAM's model. See [Term Representation](term-representation.md) for value encoding details.

```
LONA PROCESS MEMORY LAYOUT
════════════════════════════════════════════════════════════════════════════════

YOUNG HEAP (stack + young objects, single contiguous block):
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│   STACK                             FREE                    YOUNG OBJECTS   │
│   (grows down)                     SPACE                       (grows up)   │
│                                                                             │
│   ┌─────────────────────┐                          ┌─────────────────────┐  │
│   │ Frame Header (32B)  │                          │ Term (8B)           │  │
│   │ ├─ return_ip        │                          ├─────────────────────┤  │
│   │ ├─ chunk_addr       │                          │ Pair (16B)          │  │
│   │ ├─ caller_frame     │                          ├─────────────────────┤  │
│   │ └─ y_count          │                          │ HeapTuple (8B + N×8)│  │
│   ├─────────────────────┤                          ├─────────────────────┤  │
│   │ Y(0) (8B)           │                          │ HeapString (8B + N) │  │
│   │ Y(1) (8B)           │                          ├─────────────────────┤  │
│   │ ...                 │◄─stop              htop─►│ [next allocation]   │  │
│   └─────────────────────┘                          └─────────────────────┘  │
│            │                                                    │           │
│            ▼                                                    ▲           │
│       grows toward ──────────────────────────────────── grows toward        │
│       lower addresses                               higher addresses        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
▲                                                                             ▲
│                                                                             │
heap (low address)                                                hend (high)

Key pointers:
  heap      = base address of young heap
  hend      = end address of young heap
  htop      = heap top (allocation pointer, grows UP)
  stop      = stack pointer (grows DOWN)

GC trigger: htop >= stop (young heap exhausted)


OLD HEAP (promoted objects, separate block):
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│   PROMOTED OBJECTS (survived Minor GC)                          FREE        │
│                                                                SPACE        │
│   ┌─────────────────────┬─────────────────────┬─────────────────────┐       │
│   │ [promoted GC N-2]   │ [promoted GC N-1]   │ [promoted GC N]     │◄──────│
│   └─────────────────────┴─────────────────────┴─────────────────────┘       │
│                                                                  ▲          │
│                                                              old_htop       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
▲                                                                             ▲
│                                                                             │
old_heap                                                              old_hend

Never touched during Minor GC. Collected only during Major GC.
```

### Term Representation

Lona uses BEAM-style **tagged words** (8 bytes) for memory efficiency. See [Term Representation](term-representation.md) for complete details.

```
TERM FORMAT (8 bytes)
════════════════════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────┬──────┐
│                     Payload (62 bits)                           │ Tag  │
│                                                                 │ (2b) │
└─────────────────────────────────────────────────────────────────┴──────┘

Primary Tags:
  00 = HEADER (heap object marker, only on heap)
  01 = LIST (pointer to pair)
  10 = BOXED (pointer to heap object with header)
  11 = IMMEDIATE (nil, bool, small int, symbol, keyword)
```

**GC Classification**:
- **Immediate** (tag `11`): No GC action needed
- **List** (tag `01`): Points to 16-byte pair (no header)
- **Boxed** (tag `10`): Points to heap object with 8-byte header
- Symbols/keywords: Interned in realm, not on process heap (not copied)

### Heap Object Layouts

All heap objects (except pairs) have an 8-byte header word. See [Term Representation](term-representation.md) for full details.

```
HEAP OBJECT LAYOUTS
════════════════════════════════════════════════════════════════════════════════

Header word format (8 bytes):
┌───────────────────────────────────────────┬────────────────────┬──────┐
│              Arity / Size (54 bits)       │   Object Tag (8b)  │  00  │
└───────────────────────────────────────────┴────────────────────┴──────┘

Pair (NO header - identified by PAIR tag on pointer):
┌────────────────────────────────────┬────────────────────────────────────┐
│ Head: Term (8B)                    │ Rest: Term (8B)                    │
└────────────────────────────────────┴────────────────────────────────────┘
Total: 16 bytes

Tuple (header + elements):
┌────────────────────────────────────┬────────────────────────────────────┐
│ Header (arity=N, tag=TUPLE) (8B)   │ elements: [Term; N]                │
└────────────────────────────────────┴────────────────────────────────────┘
Total: 8 + N × 8 bytes

String (header + UTF-8 data):
┌────────────────────────────────────┬────────────────────────────────────┐
│ Header (arity=len, tag=STRING) (8B)│ UTF-8 bytes (aligned to 8)         │
└────────────────────────────────────┴────────────────────────────────────┘
Total: 8 + align8(len) bytes

Map (header + entries term):
┌────────────────────────────────────┬────────────────────────────────────┐
│ Header (tag=MAP) (8B)              │ entries: Term (8B)                 │
└────────────────────────────────────┴────────────────────────────────────┘
Total: 16 bytes

Closure (header + function + captures):
┌────────────────────────────────────┬────────────────────────────────────┐
│ Header (arity=N, tag=CLOSURE) (8B) │ function: Term (8B)                │
├────────────────────────────────────┴────────────────────────────────────┤
│ captures: [Term; N]                                                      │
└─────────────────────────────────────────────────────────────────────────┘
Total: 16 + N × 8 bytes
```

---

## Minor GC (Young Generation Collection)

Minor GC runs when the young heap is exhausted. It collects only young objects, promoting live ones to the old heap.

### Trigger Condition

```
MINOR GC TRIGGER
════════════════════════════════════════════════════════════════════════════════

fn alloc(&mut self, size: usize) -> Option<Vaddr> {
    let aligned_size = align_up(size, 8);
    let new_htop = self.htop + aligned_size;

    if new_htop >= self.stop {
        // Young heap exhausted - trigger Minor GC
        self.minor_gc()?;

        // Retry allocation after GC
        let new_htop = self.htop + aligned_size;
        if new_htop >= self.stop {
            // Still not enough space - need heap growth
            self.grow_heap(aligned_size)?;
        }
    }

    let addr = self.htop;
    self.htop = new_htop;
    Some(addr)
}
```

### Minor GC Algorithm

**IMPORTANT**: The stack does NOT move during minor GC. Only young heap objects (between `heap` and `htop`) are copied to old heap. The stack (between `stop` and `hend`) stays in place.

```
MINOR GC ALGORITHM
════════════════════════════════════════════════════════════════════════════════

Phase 1: Prepare
────────────────────────────────────────────────────────────────────────────────
  old_scan = old_htop              // Track where we start copying in old heap

Phase 2: Scan Roots
────────────────────────────────────────────────────────────────────────────────
  For each root:
    - X registers (from worker)
    - Y registers (walk stack frames)
    - Process-bound var bindings
    - Process execution state (chunk_addr)
    - Heap fragments (mbuf_list)

  For each root term:
    If needs_tracing(term) && is_in_young_heap(term.to_ptr()):
      new_addr = copy_to_old_heap(term)
      update root to point to new_addr

Phase 3: Scan Copied Objects (Cheney's scan)
────────────────────────────────────────────────────────────────────────────────
  scan = old_scan
  While scan < old_htop:
    object = read_object_at(scan)
    For each pointer field in object:
      If needs_tracing(field) && is_in_young_heap(field.to_ptr()):
        new_addr = copy_or_forward(field)
        update field to new_addr
    scan += object.size()

Phase 4: Sweep MSO List
────────────────────────────────────────────────────────────────────────────────
  For each MSO entry (ProcBin reference):
    If ProcBin in young heap:
      If has forwarding pointer:
        Update MSO entry to new ProcBin address  // CRITICAL
      Else (dead):
        Decrement binary refcount
        Remove from MSO list
    Else (in old heap):
      Keep entry (implicitly live during minor GC)

Phase 5: Reset Young Heap
────────────────────────────────────────────────────────────────────────────────
  htop = heap                      // Young heap space now free
  // Note: stop is NOT changed - stack stays at top of young heap block

Phase 6: Free Heap Fragments
────────────────────────────────────────────────────────────────────────────────
  For each heap fragment in mbuf_list:
    Free fragment memory
  mbuf_list = nil
```

The young heap block is **reused**, not deallocated. Only `htop` resets to `heap`.

### Copy and Forwarding

```
COPY AND FORWARDING
════════════════════════════════════════════════════════════════════════════════

fn copy_to_old_heap(&mut self, addr: Vaddr) -> Vaddr {
    // Check for existing forwarding pointer
    let header = read_u64(addr);
    if is_forwarding_pointer(header) {
        return extract_forward_addr(header);
    }

    // Calculate object size based on type
    let size = object_size_at(addr);

    // Ensure old heap has space
    if self.old_htop + size > self.old_hend {
        self.grow_old_heap(size);
    }

    // Copy object to old heap
    let new_addr = self.old_htop;
    copy_bytes(addr, new_addr, size);
    self.old_htop += size;

    // Leave forwarding pointer in original location
    write_forwarding_pointer(addr, new_addr);

    new_addr
}

FORWARDING POINTER FORMAT:
┌──────────────────────────────────────────────────────────────────────────────┐
│ Original object header replaced with forwarding header (8 bytes):            │
│ ┌────────────────────────────────────────────────────────────────────────┐   │
│ │ HEADER: new_address >> 3, tag=FORWARD (0xFF)                   (8B)    │   │
│ └────────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│ Detection: header.object_tag() == 0xFF                                       │
│ Address extraction: (header >> 10) << 3  // Re-align from arity field        │
│                                                                              │
│ This uses the same 8-byte header word format as all boxed objects.           │
│ See term-representation.md for encoding details.                             │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Root Finding

```
ROOT FINDING
════════════════════════════════════════════════════════════════════════════════

fn scan_roots(&mut self, worker: &Worker) {
    // 1. X Registers (on worker, shared by all processes)
    // X registers hold Terms (8 bytes each)
    for i in 0..X_REG_COUNT {
        self.trace_term(&mut worker.x_regs[i]);
    }

    // 2. Y Registers (walk stack frames)
    // Y registers are Terms stored on the stack
    let mut frame_ptr = self.frame_base;
    while let Some(fp) = frame_ptr {
        let frame = read_frame_header(fp);

        for y in 0..frame.y_count {
            let y_addr = fp - FRAME_HEADER_SIZE - (y + 1) * TERM_SIZE;  // TERM_SIZE = 8
            self.trace_term_at(y_addr);
        }

        frame_ptr = frame.caller_frame_base;
    }

    // 3. Process-bound var bindings
    // binding_values is array of Terms
    for i in 0..self.binding_len {
        self.trace_term(&mut self.binding_values[i]);
    }

    // 4. Mailbox and heap fragments
    // Mailbox messages contain Terms that must be traced
    self.trace_mailbox();
    self.trace_heap_fragments();
}

fn trace_term(&mut self, term: &mut Term) {
    // Immediates (tag 11) need no tracing - value is inline
    if term.is_immediate() {
        return;
    }

    // Get pointer (LIST tag 01 or BOXED tag 10)
    let addr = term.to_ptr();

    // Skip realm-resident values (symbols, keywords, vars, namespaces)
    // These are in code region, not process heap
    if self.is_realm_address(addr) {
        return;
    }

    // Skip if already in old heap
    if self.is_in_old_heap(addr) {
        return;
    }

    // Must be in young heap - copy to old
    let new_addr = self.copy_to_old_heap(addr);

    // Update term to point to new location, preserving tag
    if term.is_list() {
        *term = Term::list(new_addr as *const Pair);
    } else {
        *term = Term::boxed(new_addr as *const Header);
    }
}
```

---

## Major GC (Full Collection)

Major GC collects both young and old heaps, reclaiming dead objects in the old generation.

### Trigger Conditions

```
MAJOR GC TRIGGERS
════════════════════════════════════════════════════════════════════════════════

1. Old heap space exhausted during Minor GC
   - Minor GC tries to promote, but old_htop + size > old_hend
   - Major GC compacts old heap to make room

2. After N minor GCs without fullsweep (configurable)
   - Prevents dead old objects from accumulating indefinitely
   - Default: fullsweep_after = 65535 (or process option)

3. Explicit GC request
   - (garbage-collect :full) from Lonala code

4. Old heap utilization too low
   - After Minor GC, if old heap is < 25% utilized
   - Indicates significant old generation garbage
```

### Major GC Algorithm

```
MAJOR GC ALGORITHM
════════════════════════════════════════════════════════════════════════════════

Phase 1: Allocate New Heaps
────────────────────────────────────────────────────────────────────────────────
  Calculate required size based on live data estimate
  new_young_heap = allocate_heap(calculated_size)
  new_old_heap = allocate_heap(initial_old_size)

  new_htop = new_young_heap.base   // Start copying to new young
  new_stop = new_young_heap.end    // Stack will grow down from here

Phase 2: Copy Stack to New Young Heap
────────────────────────────────────────────────────────────────────────────────
  Copy stack frames to top of new young heap
  Update new_stop accordingly
  Update frame pointers within copied stack

Phase 3: Scan Roots (into new young heap)
────────────────────────────────────────────────────────────────────────────────
  For each root value:
    If value.is_heap_pointer():
      If in old_heap OR in young_heap:
        new_addr = copy_to_new_young(value.addr)
        update root

Phase 4: Cheney's Scan (single pass)
────────────────────────────────────────────────────────────────────────────────
  scan = new_young_heap.base
  While scan < new_htop:
    For each pointer in object at scan:
      If in old_heap OR in young_heap:
        new_addr = copy_or_forward(addr)
        update pointer
    scan += object.size()

Phase 5: Swap Heaps
────────────────────────────────────────────────────────────────────────────────
  Free old young heap
  Free old old heap

  heap = new_young_heap.base
  hend = new_young_heap.end
  htop = new_htop
  stop = new_stop

  old_heap = new_old_heap.base
  old_hend = new_old_heap.end
  old_htop = new_old_heap.base   // Empty, ready for future promotions

Phase 6: Cleanup
────────────────────────────────────────────────────────────────────────────────
  Free all heap fragments
  Decrement binary reference counts (see Binary Handling)
```

### Visualization

```
MAJOR GC VISUALIZATION
════════════════════════════════════════════════════════════════════════════════

BEFORE Major GC:
┌─────────────────────────────────────────────────────────────────────────────┐
│  Young Heap: [STACK][young live][young dead]                                │
│  Old Heap: [old live][old dead][old live][old dead][old live]               │
│                      ▲ Dead objects accumulate in old heap                  │
└─────────────────────────────────────────────────────────────────────────────┘

AFTER Major GC:
┌─────────────────────────────────────────────────────────────────────────────┐
│  New Young Heap: [STACK][all live data compacted][    FREE     ]            │
│  New Old Heap: [                    EMPTY                      ]            │
│                                                                             │
│  All live data now in young heap, fresh start                               │
│  Dead objects from both generations reclaimed                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Heap Growth and Shrinking

### Growth Strategy

```
LONA HEAP GROWTH
════════════════════════════════════════════════════════════════════════════════

GROWTH SEQUENCE (matching BEAM):
────────────────────────────────────────────────────────────────────────────────

const HEAP_SIZES: [usize; 24] = [
    // Phase 1: Fibonacci-like (words, multiply by 8 for bytes)
    233, 377, 610, 987, 1597, 2584, 4181, 6765, 10946, 17711,
    28657, 46368, 75025, 121393, 196418, 317811, 514229, 832040,
    // Phase 2: ~20% growth kicks in after this
    1346269, 2178309, 3524578, 5702887, 9227465, 14930352
];

// After 14930352 words (~119 MB), grow by 20%
const LARGE_HEAP_GROWTH_RATE: f64 = 1.20;

fn next_heap_size(current: usize, needed: usize) -> usize {
    // Find smallest size that fits
    for &size in HEAP_SIZES.iter() {
        if size >= needed {
            return size;
        }
    }

    // Beyond table: 20% growth
    let mut size = HEAP_SIZES[HEAP_SIZES.len() - 1];
    while size < needed {
        size = (size as f64 * LARGE_HEAP_GROWTH_RATE) as usize;
    }
    size
}

INITIAL SIZES:
────────────────────────────────────────────────────────────────────────────────
  Young heap: 233 words = 1864 bytes ≈ 2 KB
  Old heap: 64 words = 512 bytes

These small defaults enable millions of lightweight processes.
Heaps grow only when needed.
```

### Shrinking Strategy

```
HEAP SHRINKING
════════════════════════════════════════════════════════════════════════════════

After Major GC, consider shrinking if heap is underutilized:

fn maybe_shrink_heap(&mut self, live_data_size: usize) {
    let current_size = self.hend - self.heap;
    let utilization = live_data_size as f64 / current_size as f64;

    if utilization < 0.25 {
        // Less than 25% utilized - try to shrink
        let target_size = next_heap_size(0, live_data_size * 2);

        if target_size < current_size {
            // Allocate smaller heap, copy data
            self.resize_heap(target_size);
        }
    }
}

Prevents memory waste from processes that had temporary spikes.
```

---

## Large Binary Handling

Lona uses reference-counted binaries in a realm-wide pool, following BEAM's model.

### Binary Classification

```
BINARY CLASSIFICATION
════════════════════════════════════════════════════════════════════════════════

SMALL BINARY (< 64 bytes):
- Stored directly in process heap as HeapString
- Copied when sent in messages
- GC'd with other heap objects
- Example: (def name "hello") → "hello" on process heap

LARGE BINARY (>= 64 bytes):
- Binary data stored in realm-wide binary heap
- ProcBin (reference) stored in process heap
- Reference counted (shared between processes WITHIN SAME REALM)
- NOT copied when sent in messages (intra-realm)
- Example: (slurp "large-file.txt") → ProcBin pointing to binary heap

CROSS-REALM BINARY SHARING
────────────────────────────────────────────────────────────────────────────────
Large binaries CAN be shared across realm boundaries via capability-based
frame sharing. This enables zero-copy binary transfer between realms.

When sending a message containing a large binary to another realm:
  1. The sender grants the receiver a read capability to the binary's frame(s)
  2. The receiving realm maps those frames into its address space
  3. A new ProcBin is created pointing to the shared memory

Security is maintained through seL4's capability system:
  - The receiver only gets read access (no modification)
  - The capability can be revoked by the sender's realm
  - Memory is reclaimed only when all capabilities are revoked

This follows BEAM's zero-copy philosophy while maintaining capability security.
```

### ProcBin Structure

```
PROCBIN AND REFC BINARY
════════════════════════════════════════════════════════════════════════════════

Process Heap:                     Realm Binary Heap:
┌─────────────────────────────┐   ┌─────────────────────────────────────────┐
│ ProcBin                     │   │ RefcBinary                              │
│ ┌─────────────────────────┐ │   │ ┌─────────────────────────────────────┐ │
│ │ tag: PROCBIN            │ │   │ │ refcount: AtomicU32                 │ │
│ │ binary_addr: Vaddr ─────┼─┼──►│ │ size: u32                           │ │
│ │ offset: u32             │ │   │ │ data: [u8; size]                    │ │
│ │ size: u32               │ │   │ └─────────────────────────────────────┘ │
│ └─────────────────────────┘ │   │                                         │
│ Size: 24 bytes              │   │ Size: 8 + size bytes                    │
└─────────────────────────────┘   └─────────────────────────────────────────┘

Sub-binary (view into existing binary):
┌─────────────────────────────┐
│ SubBin                      │
│ ┌─────────────────────────┐ │
│ │ tag: SUBBIN             │ │
│ │ original: Vaddr ────────┼─┼──► RefcBinary (increments refcount)
│ │ offset: u32             │ │
│ │ size: u32               │ │
│ └─────────────────────────┘ │
│ Size: 24 bytes              │
└─────────────────────────────┘
```

### MSO (Mark-Sweep Object) List

```
MSO LIST FOR BINARY GC
════════════════════════════════════════════════════════════════════════════════

Each process tracks off-heap references in an MSO list:

struct Process {
    // ... other fields ...
    mso_list: Option<Vaddr>,  // Head of MSO linked list
}

struct MsoEntry {
    next: Option<Vaddr>,
    object: Vaddr,           // ProcBin or other off-heap reference
}

DURING MINOR/MAJOR GC:
────────────────────────────────────────────────────────────────────────────────

fn sweep_mso_list(&mut self) {
    let mut prev: Option<Vaddr> = None;
    let mut current = self.mso_list;

    while let Some(entry_addr) = current {
        let entry = read_mso_entry(entry_addr);
        let procbin_addr = entry.object;

        // Check if ProcBin was copied (has forwarding pointer)
        if has_forwarding_pointer(procbin_addr) {
            // ProcBin is still live - update next pointer and continue
            prev = Some(entry_addr);
        } else {
            // ProcBin is dead - decrement binary refcount
            let binary_addr = read_procbin(procbin_addr).binary_addr;
            let new_refcount = decrement_refcount(binary_addr);

            if new_refcount == 0 {
                // Binary is garbage - free from realm binary heap
                free_refc_binary(binary_addr);
            }

            // Remove entry from MSO list
            if let Some(p) = prev {
                write_mso_next(p, entry.next);
            } else {
                self.mso_list = entry.next;
            }
        }

        current = entry.next;
    }
}
```

### Virtual Binary Heap

```
VIRTUAL BINARY HEAP
════════════════════════════════════════════════════════════════════════════════

Each process tracks cumulative off-heap binary size to trigger early GC:

struct Process {
    // ... other fields ...
    vbin_size: usize,        // Virtual binary heap size
    vbin_limit: usize,       // Threshold for early GC
}

fn allocate_binary(&mut self, size: usize) -> Result<Vaddr, Error> {
    if size < LARGE_BINARY_THRESHOLD {
        // Small binary - allocate on process heap
        return self.alloc_string(size);
    }

    // Large binary - allocate in realm binary heap
    let binary_addr = self.realm.alloc_refc_binary(size)?;
    let procbin_addr = self.alloc_procbin(binary_addr, size)?;

    // Update virtual binary heap
    self.vbin_size += size;

    // Check if we should trigger early GC
    if self.vbin_size > self.vbin_limit {
        self.minor_gc();
        // GC will sweep MSO list, potentially freeing binaries
        // and reducing vbin_size
    }

    Ok(procbin_addr)
}

Virtual binary heap ensures that processes creating many large binaries
don't accumulate unbounded binary memory before GC runs.
```

---

## Heap Fragments (Message Passing)

When sending messages, data must be copied to the receiver's heap. Heap fragments handle contention.

### Fragment Creation

```
HEAP FRAGMENT CREATION
════════════════════════════════════════════════════════════════════════════════

SCENARIO: Process A sends message to Process B

OPTION 1: Direct allocation (common case)
────────────────────────────────────────────────────────────────────────────────
  A acquires B's heap lock (brief trylock)
  A deep copies message directly into B's young heap
  A releases lock
  Message immediately part of B's heap

OPTION 2: Heap fragment (contention case)
────────────────────────────────────────────────────────────────────────────────
  A cannot acquire B's heap lock (B is busy)
  A allocates heap fragment from worker's allocator
  A deep copies message into fragment
  A atomically links fragment to B's mbuf_list
  A continues without waiting

HEAP FRAGMENT STRUCTURE:
┌─────────────────────────────────────────────────────────────────────────────┐
│ HeapFragment                                                                │
│ ┌─────────────────┬─────────────────┬──────────────────────────────────┐    │
│ │ next: *Fragment │ size: usize     │ MESSAGE DATA (Values, objects)   │    │
│ └─────────────────┴─────────────────┴──────────────────────────────────┘    │
│                                                                             │
│ Linked to receiver's mbuf_list                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Fragment Consolidation

```
FRAGMENT CONSOLIDATION DURING GC
════════════════════════════════════════════════════════════════════════════════

Heap fragments are treated as part of young generation during GC:

fn minor_gc(&mut self) {
    // Fragments are additional roots
    for fragment in self.mbuf_list.iter() {
        for value in fragment.values() {
            self.trace_value(value);
        }
    }

    // After copying live data to old heap...

    // Free all fragments (data is now in old heap)
    for fragment in self.mbuf_list.drain() {
        self.worker.allocator.free(fragment);
    }
}

BEFORE GC:
  Main heap: [stack][objects]
  Fragment 1: [msg from A]
  Fragment 2: [msg from C]

AFTER GC:
  Main heap: [stack][      FREE      ]
  Old heap: [promoted objects + live message data]
  Fragments: freed
```

---

## Memory Allocation Architecture

### Per-Worker Allocators

```
PER-WORKER ALLOCATOR ARCHITECTURE
════════════════════════════════════════════════════════════════════════════════

Each worker has an independent allocator instance for lock-free allocation:

                         PROCESS POOL MEMORY REGION
                         (from realm's VSpace)
                                      │
            ┌─────────────────────────┼─────────────────────────┐
            │                         │                         │
            ▼                         ▼                         ▼
   ┌─────────────────┐      ┌─────────────────┐      ┌─────────────────┐
   │    WORKER 0     │      │    WORKER 1     │      │    WORKER 2     │
   │   ALLOCATOR     │      │   ALLOCATOR     │      │   ALLOCATOR     │
   │                 │      │                 │      │                 │
   │  ┌───────────┐  │      │  ┌───────────┐  │      │  ┌───────────┐  │
   │  │ Carrier 0 │  │      │  │ Carrier 0 │  │      │  │ Carrier 0 │  │
   │  │ [blocks]  │  │      │  │ [blocks]  │  │      │  │ [blocks]  │  │
   │  ├───────────┤  │      │  ├───────────┤  │      │  ├───────────┤  │
   │  │ Carrier 1 │  │      │  │ Carrier 1 │  │      │  │ Carrier 1 │  │
   │  │ [blocks]  │  │      │  │ [blocks]  │  │      │  │ [blocks]  │  │
   │  └───────────┘  │      │  └───────────┘  │      └───────────┘  │  │
   │                 │      │                 │      │                 │
   │   LOCK-FREE     │      │   LOCK-FREE     │      │   LOCK-FREE     │
   └────────┬────────┘      └────────┬────────┘      └────────┬────────┘
            │                        │                        │
            ▼                        ▼                        ▼
       Processes on            Processes on             Processes on
        Worker 0                Worker 1                 Worker 2

Allocations within a worker require NO locks (common case).
Coordination only needed for:
- Getting new carriers from LMM (IPC)
- Carrier migration between workers (rare)
```

### Carrier Structure

```
CARRIER STRUCTURE
════════════════════════════════════════════════════════════════════════════════

MULTIBLOCK CARRIER (typical, ~1 MB):
┌─────────────────────────────────────────────────────────────────────────────┐
│  HEADER  │ Block 0  │ Block 1  │   FREE   │ Block 2  │     FREE            │
│ (meta)   │ (P1 heap)│ (P7 heap)│          │ (P3 heap)│                     │
└──────────┴──────────┴──────────┴──────────┴──────────┴─────────────────────┘

Multiple process heaps share a carrier.
Free list tracks available blocks.

SINGLEBLOCK CARRIER (for large heaps):
┌─────────────────────────────────────────────────────────────────────────────┐
│  HEADER  │                    SINGLE LARGE BLOCK                            │
│ (meta)   │                    (one process's heap)                          │
└──────────┴──────────────────────────────────────────────────────────────────┘

Dedicated carrier for processes with very large heaps (> threshold).
```

### Requesting Memory from LMM

```
MEMORY REQUEST FLOW
════════════════════════════════════════════════════════════════════════════════

When worker's allocator needs more carriers:

1. GC completes but still needs more space
2. Worker calls lmm_request_pages(ProcessPool, count)
3. IPC to Lona Memory Manager:
   - Message: AllocPages { region: ProcessPool, count: N }
   - LMM checks realm quota
   - LMM retypes Untyped → Frames
   - LMM maps frames into realm's VSpace
   - Response: { status: Ok, vaddr: 0x... } or { status: OOM }

4. Worker receives response:
   - Success: Create new carrier from mapped pages
   - OOM: Return error to process (RuntimeError::OutOfMemory)

This explicit IPC model (vs. page faults) provides:
- Predictable latency
- Graceful OOM handling
- Clear quota enforcement
```

---

## GC Interface

### Process API

```
PROCESS GC API
════════════════════════════════════════════════════════════════════════════════

struct Process {
    /// Trigger minor GC if needed before allocation
    fn alloc(&mut self, size: usize) -> Option<Vaddr>;

    /// Force minor GC
    fn minor_gc(&mut self) -> GcResult;

    /// Force major GC (fullsweep)
    fn major_gc(&mut self) -> GcResult;

    /// Check if GC should run
    fn needs_gc(&self) -> bool {
        self.htop >= self.stop || self.vbin_size > self.vbin_limit
    }

    /// Get GC statistics
    fn gc_stats(&self) -> GcStats;
}

struct GcStats {
    minor_gcs: u64,
    major_gcs: u64,
    total_reclaimed: u64,
    last_gc_time_us: u64,
    heap_size: usize,
    heap_used: usize,
    old_heap_size: usize,
    old_heap_used: usize,
}

enum GcResult {
    Success { reclaimed: usize },
    HeapGrown { new_size: usize },
    OutOfMemory,
}
```

### Lonala Intrinsics

```lonala
;; Force garbage collection
(garbage-collect)           ; Minor GC
(garbage-collect :full)     ; Major GC (fullsweep)

;; Get GC statistics for current process
(gc-stats)
;; Returns:
;; %{:minor-gcs 42
;;   :major-gcs 3
;;   :heap-size 32768
;;   :heap-used 12456
;;   :old-heap-size 16384
;;   :old-heap-used 8192}

;; Configure GC parameters (per-process)
(spawn-opt worker-fn {:min-heap-size 8192
                      :fullsweep-after 1000})
```

---

## Differences from BEAM

### Summary Table

| Aspect | BEAM | Lona | Rationale |
|--------|------|------|-----------|
| **Memory source** | OS (malloc/mmap) | seL4 via LMM IPC | Capability-based allocation |
| **Heap allocation** | Per-scheduler allocator | Per-worker allocator | Same design, different terminology |
| **Page faults** | Transparent (OS handles) | Explicit IPC for process pool | seL4 MCS timing; predictable latency |
| **Binary heap** | Global, reference counted | Per-realm, cross-realm sharing via capabilities | Zero-copy with capability security |
| **Term representation** | Tagged words (8 bytes) | Tagged words (8 bytes) | Same approach as BEAM |
| **Header/forwarding** | Header word, inline forward | Header word, inline forward | Same approach as BEAM |
| **Heap fragments** | Off-heap allocation | Worker allocator | Same concept, different source |
| **Inter-process messages** | Deep copy | Deep copy | Identical semantics |
| **Large binaries** | Refc in global heap | Refc in realm heap + cross-realm caps | Zero-copy philosophy preserved |
| **Scheduler integration** | Reduction counting | Reduction counting | Identical |

### Key Adaptations

```
LONA-SPECIFIC ADAPTATIONS
════════════════════════════════════════════════════════════════════════════════

1. EXPLICIT MEMORY REQUESTS
────────────────────────────────────────────────────────────────────────────────
   BEAM: Heap growth via OS transparent page faults
   Lona: Heap growth via IPC to LMM

   Reason: seL4 MCS scheduling + page faults = complex timing.
           Explicit IPC provides predictable latency and error handling.

2. REALM-SCOPED BINARY HEAP WITH CROSS-REALM SHARING
────────────────────────────────────────────────────────────────────────────────
   BEAM: Single global binary heap for entire VM
   Lona: Separate binary heap per realm, with capability-based cross-realm sharing

   Mechanism: Binaries CAN be shared across realms via seL4 frame capabilities.
              The sender grants read capabilities to binary frames; receiver
              maps them into its address space.

   Implication: Both intra-realm and inter-realm binary sharing are zero-copy.
                Security is maintained through capability-based access control.

3. WORKER ALLOCATOR SOURCE
────────────────────────────────────────────────────────────────────────────────
   BEAM: Carriers allocated via OS malloc
   Lona: Carriers allocated via LMM IPC (Untyped → Frames)

   Same architecture (per-scheduler/worker allocators, carriers, blocks),
   different underlying memory source.

4. ISOLATED VSPACES WITH CONTROLLED SHARING
────────────────────────────────────────────────────────────────────────────────
   BEAM: Single Erlang VM, all processes share same memory space
   Lona: Each realm has isolated VSpace

   Consequence: Cross-realm messages require serialization/deserialization
                for non-binary data. Large binaries can be shared zero-copy
                via capability-based frame sharing.

   This provides security isolation while preserving BEAM's zero-copy binary
   sharing philosophy through seL4's capability model.
```

---

## Implementation Considerations

### GC-Safe Points

```
GC-SAFE POINTS
════════════════════════════════════════════════════════════════════════════════

GC can only occur at safe points where all roots are identifiable:

SAFE POINTS:
- Before/after function calls
- At allocation sites (obvious - allocation triggers GC)
- At message send/receive
- At explicit yield points

NOT SAFE (GC cannot occur):
- Mid-instruction with partial state
- During native function execution
- While constructing compound values

The bytecode interpreter naturally hits safe points between instructions.
Allocation is the primary GC trigger.
```

### Stack Scanning

```
STACK SCANNING DETAILS
════════════════════════════════════════════════════════════════════════════════

FRAME LAYOUT (reminder):
stop (after ALLOCATE)
┌──────────────────────────────────┐
│ Y(0)             (8 bytes)       │ ← stop + 0
│ Y(1)             (8 bytes)       │ ← stop + 8
│ ...                              │
│ Y(N-1)           (8 bytes)       │ ← stop + (N-1) × 8
├──────────────────────────────────┤ ← frame_base
│ return_ip        (u64)           │ + 0
│ chunk_addr       (u64)           │ + 8
│ caller_frame_base(u64)           │ + 16
│ y_count          (u64)           │ + 24
└──────────────────────────────────┘

Y_REGISTER_SIZE = 8 bytes (size of Term)
FRAME_HEADER_SIZE = 32 bytes

SCANNING ALGORITHM:
fn scan_stack(&mut self) {
    let mut fp = self.frame_base;

    while let Some(frame_ptr) = fp {
        // Read frame header
        let header = read_frame_header(frame_ptr);

        // Scan Y registers (above frame header, going up)
        for i in 0..header.y_count {
            let y_addr = frame_ptr - FRAME_HEADER_SIZE - (i + 1) * Y_REGISTER_SIZE;
            let term = read_term(y_addr);

            if let Some(new_term) = self.trace_and_copy(term) {
                write_term(y_addr, new_term);
            }
        }

        // Move to caller's frame
        fp = if header.caller_frame_base != 0 {
            Some(header.caller_frame_base)
        } else {
            None
        };
    }
}
```

### Initialization Safety

```
Y REGISTER INITIALIZATION
════════════════════════════════════════════════════════════════════════════════

Y registers must be GC-safe at all times. Two approaches:

1. ALLOCATE_ZERO (opcode 14):
   - Allocates Y registers AND initializes to nil
   - Safe for GC at any point
   - Slight overhead for initialization

2. ALLOCATE (opcode 13):
   - Allocates Y registers, UNINITIALIZED
   - Compiler must ensure all Y regs written before any GC-possible operation
   - More efficient but requires compiler discipline

Current implementation uses ALLOCATE_ZERO for safety.
Optimization: switch to ALLOCATE once liveness analysis ensures safety.
```

---

## Summary

| Aspect | Description |
|--------|-------------|
| **Model** | Per-process generational semi-space copying collector |
| **Generations** | Young (frequent) and Old (rare) |
| **Algorithm** | Cheney's copying algorithm |
| **Trigger** | Young heap exhaustion (htop >= stop) |
| **Minor GC pause** | 10-100 µs typical |
| **Major GC pause** | 100 µs - few ms |
| **Heap growth** | Fibonacci sequence, then 20% |
| **Large binaries** | Reference counted in realm binary heap |
| **Memory source** | Explicit IPC to Lona Memory Manager |
| **Allocator** | Per-worker, lock-free for common case |

This design provides soft real-time properties through per-process GC with no global pauses, while adapting BEAM's proven approach to seL4's capability-based memory model.

---

## References

- [BEAM Garbage Collection (Erlang/OTP Documentation)](https://www.erlang.org/doc/apps/erts/garbagecollection.html)
- [Erlang GC Details (Hamidreza Soleimani)](https://hamidreza-s.github.io/erlang%20garbage%20collection%20memory%20layout%20soft%20realtime/2015/08/24/erlang-garbage-collection-details-and-why-it-matters.html)
- [The BEAM Book](https://blog.stenmans.org/theBeamBook/)
- [Process Model](process-model.md) - Lona's process and heap structure
- [Realm Memory Layout](realm-memory-layout.md) - Memory regions and allocation
- [System Architecture](system-architecture.md) - LMM and IPC
