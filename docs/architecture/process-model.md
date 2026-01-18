# Process Model

This document covers Lonala processes: lightweight execution units within realms, their memory model, scheduling, message passing, and garbage collection.

## Process Characteristics

Lonala processes are modeled after BEAM/Erlang processes:

| Property | Description |
|----------|-------------|
| **Lightweight** | ~1-10 µs to spawn, minimal memory overhead (~2-4 KB initial) |
| **Isolated heap** | Each process has its own heap, no shared mutable state |
| **Mailbox** | Each process has an incoming message queue |
| **Fault isolation** | Process crash doesn't affect other processes |
| **Millions per realm** | A single realm can host millions of processes |

### Process Structure

```
PROCESS (Pure Userspace Construct)
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  Identity:                                                          │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  pid: ProcessId        - Unique within realm                │    │
│  │  parent: ProcessId     - Who spawned this process           │    │
│  │  links: Set<ProcessId> - Bidirectional crash notification   │    │
│  │  monitors: Set<MonitorRef> - Unidirectional monitoring      │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                     │
│  Execution State:                                                   │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  status: Running | Waiting | Exited                         │    │
│  │  reductions: u32       - Remaining budget (weighted costs)  │    │
│  │  total_reductions: u64 - Lifetime reductions (monitoring)   │    │
│  │  ip: usize             - Instruction pointer                │    │
│  │  x_regs: [Value; 256]  - X registers (temporaries)          │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                     │
│  Memory:                                                            │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  young_heap: contiguous block (stack + young objects)       │    │
│  │  old_heap: contiguous block (promoted objects)              │    │
│  │  htop: young heap allocation pointer (grows up)             │    │
│  │  stop: stack pointer (grows down)                           │    │
│  │  old_htop: old heap allocation pointer                      │    │
│  │  mbuf_list: linked list of heap fragments                   │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                     │
│  Communication:                                                     │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  mailbox: Queue<Message>      - Incoming messages           │    │
│  │  waiting_pattern: Option<Pattern> - What we're waiting for  │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Process Memory Model

This section describes the BEAM-style memory model used by Lonala processes. Understanding this model is essential for reasoning about performance, garbage collection, and memory usage.

### Process Memory Overview

Each process owns **two memory blocks**: a young heap and an old heap.

```
PROCESS MEMORY OVERVIEW
════════════════════════════════════════════════════════════════════════════════

Each process has two separate memory blocks:

┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  YOUNG HEAP (where most activity happens):                                  │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  STACK          │           FREE           │   YOUNG OBJECTS          │  │
│  │  (grows down)   │          SPACE           │   (grows up)             │  │
│  │       ▼         │                          │         ▲                │  │
│  │      stop       │                          │       htop               │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  OLD HEAP (promoted objects):                                               │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │  PROMOTED OBJECTS (survived minor GC)            │      FREE          │  │
│  │                                                  │      SPACE         │  │
│  │                                                  │         ▲          │  │
│  │                                                  │      old_htop      │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  HEAP FRAGMENTS (temporary, from message passing):                          │
│  ┌──────────┐  ┌──────────┐                                                 │
│  │ fragment │─►│ fragment │─► nil                                           │
│  └──────────┘  └──────────┘                                                 │
│  Consolidated into old heap during Minor GC                                 │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Young Heap Structure

The young heap is a **contiguous block** containing both stack and young objects, growing toward each other:

```
YOUNG HEAP LAYOUT
════════════════════════════════════════════════════════════════════════════════

    ┌────────────────────────────────────────────────────────────────────────┐
    │                                                                        │
    │   STACK                           FREE                    YOUNG HEAP   │
    │   (grows down)                   SPACE                    (grows up)   │
    │                                                                        │
    │   ┌─────────┐                                            ┌─────────┐   │
    │   │ frame 2 │                                            │ tuple   │   │
    │   ├─────────┤                                            ├─────────┤   │
    │   │ frame 1 │                                            │ cons    │   │
    │   ├─────────┤                                            ├─────────┤   │
    │   │ frame 0 │◄─ stop                               htop ─►│ string  │   │
    │   └─────────┘                                            └─────────┘   │
    │        │                                                      │        │
    │        ▼                                                      ▲        │
    │   grows toward ──────────────────────────────────── grows toward       │
    │   lower addresses                               higher addresses       │
    │                                                                        │
    └────────────────────────────────────────────────────────────────────────┘
    ▲                                                                        ▲
    │                                                                        │
   heap (low address)                                           hend (high address)


    Key pointers:
    ┌────────────────────────────────────────────────────────────────────────┐
    │  heap      = base address of young heap (low address)                  │
    │  hend      = end address of young heap (high address)                  │
    │  htop      = current heap top (grows UP toward hend)                   │
    │  stop      = current stack pointer (grows DOWN toward heap)            │
    │  heap_size = hend - heap (total young heap size)                       │
    └────────────────────────────────────────────────────────────────────────┘

    Out of memory condition: htop >= stop
    When this happens, Minor GC is triggered.
```

### Why This Design?

This design, taken directly from BEAM, has several advantages:

1. **Simple allocation**: Heap allocation is a pointer bump (O(1))
2. **Simple stack operations**: Push/pop are pointer adjustments
3. **Fast Minor GC**: Only young heap is scanned; old heap untouched
4. **Memory efficient**: Stack and young heap share unused space
5. **Cache friendly**: Related data is spatially close
6. **Generational hypothesis**: Most objects die young, so most GC work is in the small young heap

### Initial Heap Size

Processes start with a small heap to enable millions of lightweight processes:

```
INITIAL PROCESS MEMORY
════════════════════════════════════════════════════════════════════════════════

Default initial size: ~2 KB (configurable)

This small default allows:
- Spawning millions of processes with minimal memory
- Processes that allocate little stay small
- Heap grows only when needed

Spawn options can override:
  (spawn-opt f {:min-heap-size 8192})  ; Start with 8 KB
```

### Heap Growth Strategy

When a process needs more memory than available, the heap grows following BEAM's strategy:

```
HEAP GROWTH SEQUENCE
════════════════════════════════════════════════════════════════════════════════

Phase 1: Fibonacci-like growth (small heaps)
──────────────────────────────────────────────────────────────────────────────
  233 → 377 → 610 → 987 → 1597 → 2584 → 4181 → 6765 → 10946 → ...  (words)

  In bytes (64-bit, 8 bytes/word):
  ~2KB → ~3KB → ~5KB → ~8KB → ~13KB → ~21KB → ~33KB → ~54KB → ~87KB → ...

Phase 2: 20% growth (large heaps, after ~1 million words / ~8 MB)
──────────────────────────────────────────────────────────────────────────────
  8 MB → 9.6 MB → 11.5 MB → 13.8 MB → 16.6 MB → ...

  Rationale: Fibonacci growth is too aggressive for large heaps.
  20% growth balances memory efficiency with GC frequency.
```

### Heap Growth Mechanism

When a process runs out of memory, the entire heap is **reallocated**:

```
HEAP REALLOCATION DURING GC
════════════════════════════════════════════════════════════════════════════════

Before (heap exhausted):
┌────────────────────────────────────────────────────────────────────────────┐
│  STACK STACK STACK ◄─stop    htop─► HEAP HEAP HEAP HEAP HEAP               │
│  ├─────────────────────────│────────────────────────────────┤              │
│                        NO FREE SPACE                                       │
│                    (htop and stop have met)                                │
└────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼ GC triggered
                                    │
Step 1: Allocate NEW larger block (next size in growth sequence)
┌────────────────────────────────────────────────────────────────────────────┐
│                                                                            │
│                         NEW BLOCK (larger)                                 │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                                                                      │  │
│  │                    EMPTY - READY FOR COPYING                         │  │
│  │                                                                      │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼ Copy live data
                                    │
Step 2: Copy ONLY LIVE data from old block to new block
┌────────────────────────────────────────────────────────────────────────────┐
│                                                                            │
│  OLD BLOCK                           NEW BLOCK                             │
│  ┌─────────────────────────┐         ┌────────────────────────────────┐    │
│  │ S │ S │ D │ L │ D │ L │ │   ───►  │ S │ S │         │ L │ L │      │    │
│  │   │   │   │   │   │   │ │  copy   │   │   │  FREE   │   │   │      │    │
│  │   │   │   │   │   │   │ │  live   │   │   │         │   │   │      │    │
│  └─────────────────────────┘         └────────────────────────────────┘    │
│         │                                                                  │
│         ▼                                                                  │
│      FREED                           S = Stack frames (always live)        │
│                                      L = Live heap objects                 │
│                                      D = Dead objects (not copied)         │
└────────────────────────────────────────────────────────────────────────────┘

Key points:
- Dead objects are NOT copied (garbage collected)
- Live objects are compacted (no fragmentation)
- Old block is freed after copy completes
- Process resumes with more free space
```

### Process Isolation During GC

**Critical design property**: Each process GCs independently. No global pauses.

```
PER-PROCESS GARBAGE COLLECTION
════════════════════════════════════════════════════════════════════════════════

Process A                  Process B                  Process C
    │                          │                          │
    │ GC in progress           │ executing                │ executing
    │ ████████████████         │                          │
    │ (A is paused)            │ (B keeps running)        │ (C keeps running)
    │                          │                          │
    │                          │ GC in progress           │
    │ executing                │ ████████████████         │ executing
    │ (A keeps running)        │ (B is paused)            │ (C keeps running)
    │                          │                          │
    ▼                          ▼                          ▼

NO COORDINATION between processes.
GC of Process A has ZERO impact on Process B or C.
Only the process being GCd is paused.

This is the key to Erlang/BEAM's soft real-time properties:
- System remains responsive during GC
- GC pauses are per-process (microseconds to milliseconds)
- No stop-the-world pauses
```

---

## Memory Allocation Architecture

This section describes how memory is allocated for process heaps, following BEAM's per-scheduler allocator design.

### Per-Worker Allocator Instances

To avoid lock contention in multi-worker realms, each worker has its **own allocator instance**:

```
PER-WORKER ALLOCATOR ARCHITECTURE
════════════════════════════════════════════════════════════════════════════════

                         PROCESS POOL MEMORY REGION
                         (from realm's VSpace)
                                      │
            ┌─────────────────────────┼─────────────────────────┐
            │                         │                         │
            ▼                         ▼                         ▼
   ┌─────────────────┐      ┌─────────────────┐      ┌─────────────────┐
   │    WORKER 0     │      │    WORKER 1     │      │    WORKER 2     │
   │   ALLOCATOR     │      │   ALLOCATOR     │      │   ALLOCATOR     │
   │   INSTANCE      │      │   INSTANCE      │      │   INSTANCE      │
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


Each worker's allocator is INDEPENDENT:
- Allocations are lock-free within a worker
- No contention between workers
- Processes running on a worker use that worker's allocator
```

### Carriers

Allocators manage memory in large chunks called **carriers**:

```
CARRIER STRUCTURE
════════════════════════════════════════════════════════════════════════════════

Carriers are large memory regions obtained from the process pool.
Each carrier contains multiple allocation blocks.

┌─────────────────────────────────────────────────────────────────────────────┐
│                           CARRIER (e.g., 1 MB)                              │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ HEADER │ Block 0  │ Block 1  │   FREE   │ Block 2  │     FREE        │  │
│  │        │ (P1 heap)│ (P7 heap)│          │ (P3 heap)│                 │  │
│  └────────┴──────────┴──────────┴──────────┴──────────┴─────────────────┘  │
│                                                                             │
│  Blocks within a carrier:                                                   │
│  - Allocated to individual process heaps                                    │
│  - Variable sizes (match heap size requests)                                │
│  - Freed when process terminates or heap shrinks                            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

Carrier types:
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  MULTIBLOCK CARRIER: Contains multiple smaller blocks                       │
│  - Used for typical process heaps                                           │
│  - Efficient for small to medium allocations                                │
│                                                                             │
│  SINGLEBLOCK CARRIER: Contains one large block                              │
│  - Used for very large process heaps (above threshold)                      │
│  - Dedicated carrier per large heap                                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Allocation Path (Lock-Free)

The common case - allocating memory for a process - requires no locks:

```
ALLOCATION PATH
════════════════════════════════════════════════════════════════════════════════

Process P (running on Worker W) needs to grow heap:

1. GC determines: "I need N bytes for new heap"

2. Request goes to Worker W's allocator instance
   └── This is the SAME worker running P
   └── No other worker touches this allocator
   └── LOCK-FREE operation

3. Allocator checks its carriers:
   └── Has space in existing carrier? → Allocate from carrier
   └── No space? → Get new carrier from pool

4. Return memory to process P

No locks needed for steps 1-4 (common case).

Only rare operations require coordination:
- Getting new carrier from pool (page fault handler)
- Carrier migration between workers (load balancing)
- Cross-worker deallocation (process migrated between workers)
```

### Carrier Migration

When workers become imbalanced, carriers can migrate:

```
CARRIER MIGRATION (Rare Operation)
════════════════════════════════════════════════════════════════════════════════

Scenario: Worker 0 has many free carriers, Worker 1 needs more

Worker 0 (lots of free carriers)     Worker 1 (needs carriers)
┌─────────────────────────────────┐  ┌─────────────────────────────────┐
│  Carrier A [25% used]           │  │  Carrier X [95% used]           │
│  Carrier B [10% used] ◄─────────┼──┼─ "I'll take this one"           │
│  Carrier C [50% used]           │  │                                 │
└─────────────────────────────────┘  └─────────────────────────────────┘

Process:
1. Worker 1 notices it needs more carriers
2. Worker 1 checks for "abandoned" carriers from other workers
3. Worker 0 had marked Carrier B as "abandonable" (low utilization)
4. Worker 1 adopts Carrier B (atomic pointer swap)
5. Future allocations in Worker 1 can use Carrier B

This is RARE - only happens during load imbalance.
Normal allocations remain lock-free.
```

---

## Heap Fragments (M-Bufs)

When a process cannot write directly to another process's heap, it uses **heap fragments**:

```
HEAP FRAGMENTS (M-BUFS)
════════════════════════════════════════════════════════════════════════════════

Scenario: Process A sends message to Process B

Option 1: B's heap lock available (common case)
──────────────────────────────────────────────────────────────────────────────
  A acquires B's heap lock
  A copies message DIRECTLY into B's heap
  A releases lock
  Message is immediately part of B's heap

Option 2: B's heap lock busy (contention case)
──────────────────────────────────────────────────────────────────────────────
  A cannot acquire B's heap lock
  A allocates a HEAP FRAGMENT (m-buf) outside B's main heap
  A copies message into the fragment
  A links fragment to B's fragment list (atomic operation)
  A continues without waiting

  Fragment structure:
  ┌─────────────────────────────────────────────────────────────────────────┐
  │  HEAP FRAGMENT                                                          │
  │  ┌───────────────────────────────────────────────────────────────────┐  │
  │  │ next: *Fragment │ size: usize │     MESSAGE DATA                  │  │
  │  └─────────────────┴─────────────┴───────────────────────────────────┘  │
  │                                                                         │
  │  Linked to B's mbuf_list (process field)                                │
  └─────────────────────────────────────────────────────────────────────────┘

Process B's view:
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  Main Heap Block                      Heap Fragments                        │
│  ┌─────────────────────────────┐      ┌──────────────┐                      │
│  │ STACK       │      │  HEAP  │      │ Fragment 1   │                      │
│  │             │      │        │      │ (msg from A) │                      │
│  │             │      │        │ ◄────│ next ────────┼──► Fragment 2        │
│  │             │      │        │      │              │    (msg from C)      │
│  └─────────────┴──────┴────────┘      └──────────────┘                      │
│                                                                             │
│  Fragments are considered part of young generation (above high_water)       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

Fragment consolidation (during GC):
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  Before GC:                                                                 │
│    Main heap + Fragment 1 + Fragment 2 (separate memory regions)            │
│                                                                             │
│  During GC:                                                                 │
│    Live data from main heap + fragments copied to NEW heap block            │
│                                                                             │
│  After GC:                                                                  │
│    Single contiguous heap block (fragments freed, data consolidated)        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

Benefits of heap fragments:
- Senders never block waiting for receiver's heap lock
- Reduces lock contention in high-throughput scenarios
- Fragments consolidated during normal GC (no extra passes)
```

---

## Generational Garbage Collection

Lonala uses a generational copying collector, following BEAM's design.

### Generations

```
GENERATIONAL GC MODEL
════════════════════════════════════════════════════════════════════════════════

Process heap is divided into generations by the HIGH_WATER mark:

┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │ STACK │          │ OLD GENERATION  │ YOUNG GENERATION │  FREE       │    │
│  │       │          │ (below h_water) │ (above h_water)  │             │    │
│  │ ───►  │          │                 │                  │  ◄───       │    │
│  └───────┴──────────┴─────────────────┴──────────────────┴─────────────┘    │
│      ▲                      ▲                    ▲              ▲           │
│     stop               high_water            (recent)        htop          │
│                                                                             │
│  Objects below high_water: survived at least one GC (old generation)        │
│  Objects above high_water: recently allocated (young generation)            │
│  Heap fragments: always considered young generation                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Two-Heap Architecture

Each process has two memory blocks: a **young heap** and an **old heap**:

```
PROCESS MEMORY: TWO-HEAP ARCHITECTURE
════════════════════════════════════════════════════════════════════════════════

Young Heap (stack + young objects):
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│   STACK                         FREE                    YOUNG OBJECTS       │
│   (grows down)                 SPACE                       (grows up)       │
│                                                                             │
│   ┌─────────┐                                            ┌─────────┐        │
│   │ frame 1 │                                            │ new obj │        │
│   ├─────────┤                                            ├─────────┤        │
│   │ frame 0 │◄─ stop                               htop ─►│ new obj │        │
│   └─────────┘                                            └─────────┘        │
│                                                                             │
│   Out of memory when htop >= stop → triggers Minor GC                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

Old Heap (promoted objects, separate block):
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│   PROMOTED OBJECTS                                            FREE          │
│   (survived minor GC)                                        SPACE          │
│                                                                             │
│   ┌──────────┬──────────┬──────────┬──────────┐                             │
│   │ promoted │ promoted │ promoted │          │◄─ old_htop                  │
│   │ (GC N-3) │ (GC N-2) │ (GC N-1) │          │                             │
│   └──────────┴──────────┴──────────┴──────────┘                             │
│                                                                             │
│   Never touched during Minor GC                                             │
│   Collected only during Major GC (fullsweep)                                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

Benefits of two heaps:
- Minor GC only touches young heap (fast, ~10-100 µs)
- Old heap grows independently
- Better cache behavior (young data is hot, old data is cold)
- Heap fragments are considered young (consolidated during GC)
```

### Minor GC (Young Generation Only)

Minor GC runs when the young heap is exhausted (htop meets stop). It only processes young objects.

```
MINOR GC
════════════════════════════════════════════════════════════════════════════════

Triggered when: htop >= stop (young heap exhausted)

Scope: Young heap + heap fragments only. Old heap is NOT touched.

Process:
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  1. Scan roots (stack, registers, process dictionary)                       │
│                                                                             │
│  2. For each pointer to YOUNG object:                                       │
│     - Copy object to OLD HEAP (promotion)                                   │
│     - Update pointer to new location in old heap                            │
│     - Object is now "tenured"                                               │
│                                                                             │
│  3. For pointers to OLD objects:                                            │
│     - No action needed (already in old heap)                                │
│                                                                             │
│  4. Copy live data from heap fragments to old heap                          │
│                                                                             │
│  5. Reset young heap: htop = heap_start                                     │
│     - All young space is now free for new allocations                       │
│     - Dead young objects are implicitly reclaimed                           │
│                                                                             │
│  6. Free heap fragments                                                     │
│                                                                             │
│  7. If old heap needs more space: grow old heap                             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

BEFORE Minor GC:
┌─────────────────────────────────────────────────────────────────────────────┐
│  Young Heap: [STACK STACK]◄─stop  htop─►[live][dead][live][dead]            │
│                        NO FREE SPACE (htop >= stop)                         │
│                                                                             │
│  Old Heap: [promoted][promoted][promoted][ FREE ]                           │
└─────────────────────────────────────────────────────────────────────────────┘

AFTER Minor GC:
┌─────────────────────────────────────────────────────────────────────────────┐
│  Young Heap: [STACK STACK]◄─stop      [      ALL FREE      ]◄─htop          │
│                        Young heap reset, ready for new allocations          │
│                                                                             │
│  Old Heap: [promoted][promoted][promoted][new][new][ FREE ]                 │
│                        Live young objects promoted to old heap              │
└─────────────────────────────────────────────────────────────────────────────┘

Why minor GC is fast:
- Only scans young heap (typically small)
- Most objects die young (generational hypothesis) - no copy needed
- Old heap not scanned or touched
- Typical pause: 10-100 microseconds
```

### Major GC (Fullsweep)

Major GC collects both heaps. It runs less frequently but reclaims dead objects in the old generation.

```
MAJOR GC (FULLSWEEP)
════════════════════════════════════════════════════════════════════════════════

Triggered when:
- Old heap is too large relative to live data
- After N minor GCs without fullsweep (configurable)
- Explicit (gc :full) call

Scope: BOTH young heap AND old heap

Process:
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  1. Allocate NEW young heap block (may be larger if needed)                 │
│                                                                             │
│  2. Scan ALL roots (stack, registers, process dictionary)                   │
│                                                                             │
│  3. Copy ALL live objects (from both young AND old heaps) to new heap       │
│     - Compacts all live data into contiguous space                          │
│     - Dead objects in old generation are reclaimed                          │
│                                                                             │
│  4. Free old young heap block                                               │
│                                                                             │
│  5. Free old heap block (all data now in new young heap)                    │
│                                                                             │
│  6. Allocate fresh empty old heap                                           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

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

Major GC characteristics:
- More expensive than minor GC (scans everything)
- Reclaims dead objects in old generation
- Compacts all live data (better locality)
- Typical pause: 100 microseconds to several milliseconds
- Frequency: much less often than minor GC
```

---

## Large Binary Handling

Large binaries (>64 bytes) are handled specially to avoid copying overhead:

```
LARGE BINARY HANDLING
════════════════════════════════════════════════════════════════════════════════

Small binaries (< 64 bytes):
- Stored directly in process heap
- Copied when sent in messages
- GC'd with other heap objects

Large binaries (>= 64 bytes):
- Stored in REALM-WIDE binary heap (separate from process heaps)
- Reference counted
- NOT copied when sent in messages

┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  Process A heap:              Realm Binary Heap:                            │
│  ┌───────────────┐            ┌─────────────────────────────────────────┐   │
│  │ BinaryRef ────┼───────────►│  refcount: 2                            │   │
│  │ (8 bytes)     │            │  size: 10 MB                            │   │
│  └───────────────┘            │  data: [................]               │   │
│                               └─────────────────────────────────────────┘   │
│  Process B heap:                          ▲                                 │
│  ┌───────────────┐                        │                                 │
│  │ BinaryRef ────┼────────────────────────┘                                 │
│  │ (8 bytes)     │                                                          │
│  └───────────────┘                                                          │
│                                                                             │
│  A sends binary to B:                                                       │
│  - Only BinaryRef is copied (8 bytes)                                       │
│  - refcount incremented (2)                                                 │
│  - 10 MB data NOT copied                                                    │
│                                                                             │
│  When A or B no longer references binary:                                   │
│  - refcount decremented                                                     │
│  - When refcount reaches 0, binary is freed                                 │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

Benefits:
- Zero-copy message passing for large data
- Process heaps stay small (fast GC)
- Efficient sharing of read-only data
- Maintains "no shared mutable state" (binaries are immutable)
```

---

## Scheduling

Lonala uses a hybrid cooperative/preemptive scheduling model.

### Reduction-Based Scheduling

Within a realm, processes yield cooperatively after executing a certain number of "reductions" (bytecode instructions):

```
REDUCTION COUNTING
════════════════════════════════════════════════════════════════════════════════

const MAX_REDUCTIONS: u32 = 2000;  // Tune for ~500µs (MCS budget)

fn run_process(proc: &mut Process) -> RunResult {
    while proc.reductions > 0 {
        let instruction = fetch(proc);

        match execute(proc, instruction) {
            Continue => proc.reductions -= 1,
            Yield => return RunResult::Yielded,
            Block(reason) => return RunResult::Blocked(reason),
            Exit(reason) => return RunResult::Exited(reason),
        }
    }

    // Out of reductions - yield to other processes
    RunResult::Yielded
}
```

### Scheduler Loop

```
VM SCHEDULER LOOP
════════════════════════════════════════════════════════════════════════════════

loop {
    // Pick next runnable process
    proc = run_queue.pop_front()

    if proc is None {
        // No runnable processes - check mailboxes, maybe idle
        check_timeouts()
        if run_queue.is_empty() {
            wait_for_event()  // Block until message/timeout/signal
        }
        continue
    }

    // Reset reduction counter
    proc.reductions = MAX_REDUCTIONS

    // Run the process
    result = run_process(proc)

    match result {
        Yielded =>
            // Process used its time slice, re-queue
            run_queue.push_back(proc)

        Blocked(mailbox) =>
            // Process is waiting for a message
            waiting_processes.insert(proc.pid, proc)

        Exited(reason) =>
            // Process terminated
            notify_links(proc, reason)
            notify_monitors(proc, reason)
            cleanup(proc)
    }
}
```

### Scheduling Layers

```
THREE SCHEDULING LAYERS
════════════════════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────────┐
│ Layer 3: INTRA-REALM PROCESS SCHEDULING                             │
│                                                                     │
│   Mechanism: Reduction counting + run queue                         │
│   Granularity: ~1ms time slices                                     │
│   Fairness: Round-robin among runnable processes                    │
│   Control: Lona VM (userspace)                                      │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│ Layer 2: INTRA-REALM WORKER SCHEDULING (if multiple workers)        │
│                                                                     │
│   Mechanism: Work stealing between workers                          │
│   Granularity: Per-process                                          │
│   Fairness: Load balancing across CPUs                              │
│   Control: Lona VM (userspace)                                      │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│ Layer 1: INTER-REALM SCHEDULING                                     │
│                                                                     │
│   Mechanism: seL4 MCS scheduler                                     │
│   Granularity: Per-realm CPU budgets                                │
│   Fairness: Policy-defined (min/max budgets)                        │
│   Control: seL4 kernel (hardware-enforced)                          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

Key insight: Processes cooperate within a realm (reductions),
but the kernel preempts realms (MCS budgets). A misbehaving
process can't starve its realm's other processes, and a
misbehaving realm can't starve other realms.
```

### Work Stealing (Multi-Worker)

If a realm has multiple workers (TCBs), they can steal work from each other:

```
WORK STEALING (Optional, if multiple workers per realm)
════════════════════════════════════════════════════════════════════════════════

Each worker has a local run queue (deque):
- Owner pushes/pops from BOTTOM (LIFO - cache locality)
- Thieves steal from TOP (FIFO - oldest work)

fn worker_loop(worker_id):
    loop {
        // Try local queue first
        proc = local_queue.pop_bottom()

        if proc is None {
            // Try stealing from other workers
            proc = steal_from_others(worker_id)
        }

        if proc is None {
            // All queues empty - wait for work
            wait_for_work()
            continue
        }

        run_process(proc)
    }

fn steal_from_others(thief_id) -> Option<Process>:
    // Random start to avoid thundering herd
    start = random() % num_workers

    for i in 0..num_workers:
        victim = (start + i) % num_workers
        if victim == thief_id:
            continue

        if let Some(proc) = workers[victim].queue.steal_top():
            return Some(proc)

    None
```

---

## Message Passing

Processes communicate exclusively through message passing. No shared mutable state.

### Intra-Realm Messages (Fast Path)

Messages within the same realm use deep copy to the receiver's heap:

```
INTRA-REALM MESSAGE PASSING
════════════════════════════════════════════════════════════════════════════════

Cost: ~100-500 ns (deep copy)

sender                              receiver
   │                                   │
   │  (send receiver-pid [:ok data])   │
   │                                   │
   ├──────────────────────────────────►│
   │                                   │
   │  1. Try to acquire receiver's     │
   │     heap lock                     │
   │                                   │
   │  2a. If lock acquired:            │
   │      Deep copy to receiver's heap │
   │      (direct write)               │
   │                                   │
   │  2b. If lock busy:                │
   │      Allocate heap fragment       │
   │      Copy to fragment             │
   │      Link fragment to receiver    │
   │                                   │
   │  3. Enqueue in receiver's         │
   │     mailbox (lock-free MPSC)      │
   │                                   │
   │  4. If receiver waiting,          │
   │     wake it up                    │
   │                                   │

Deep copy ensures:
- No shared mutable state
- Receiver owns message completely
- Sender can modify original after send
- GC of sender doesn't affect receiver
```

### Inter-Realm Messages (Kernel Path)

Messages between realms require seL4 IPC and serialization:

```
INTER-REALM MESSAGE PASSING
════════════════════════════════════════════════════════════════════════════════

Cost: ~1-10 µs (serialization + IPC)

Realm A                  seL4 Kernel              Realm B
   │                         │                       │
   │  (send pid [:ok data])  │                       │
   │                         │                       │
   │  1. Serialize message   │                       │
   │     to IPC buffer       │                       │
   │                         │                       │
   ├─────seL4_Call──────────►│                       │
   │                         │                       │
   │                         ├──────seL4_Recv───────►│
   │                         │                       │
   │                         │  2. Deserialize from  │
   │                         │     IPC buffer        │
   │                         │                       │
   │                         │  3. Deep copy to      │
   │                         │     receiver's heap   │
   │                         │                       │

Inter-realm IPC requires:
- Realm A has endpoint capability for Realm B
- Serialization format (TBD - likely compact binary)
- Deserialization + deep copy on receive
```

### Mailbox Implementation

Each process has a lock-free MPSC (multiple-producer, single-consumer) queue:

```
MAILBOX (Lock-Free MPSC Queue)
════════════════════════════════════════════════════════════════════════════════

struct Mailbox {
    head: AtomicPtr<Message>,  // Producers push here
    tail: *mut Message,        // Consumer pops here
}

struct Message {
    next: *mut Message,
    sender: ProcessId,
    data: Value,  // Deep-copied to receiver's heap
}

Push (any process can send):
────────────────────────────────────────────────────────────────────────────
fn push(mailbox, msg):
    msg.next = null
    prev = atomic_exchange(&mailbox.head, msg)
    prev.next = msg  // Linearization point

Pop (only owner process):
────────────────────────────────────────────────────────────────────────────
fn pop(mailbox) -> Option<Message>:
    tail = mailbox.tail
    next = tail.next

    if next is null:
        return None

    mailbox.tail = next
    return Some(next)
```

### Selective Receive

Processes can wait for messages matching a pattern:

```lonala
;; Wait for specific message pattern
(receive
  [:ok result]    (handle-success result)
  [:error reason] (handle-error reason)
  :timeout 5000   (handle-timeout))

;; Messages not matching patterns stay in mailbox
;; for later receive calls (selective receive)
```

```
SELECTIVE RECEIVE
════════════════════════════════════════════════════════════════════════════════

Process mailbox: [msg1] [msg2] [msg3] [msg4]

(receive [:ok result] ...)

1. Check msg1 against pattern [:ok result]
   - No match, skip

2. Check msg2 against pattern [:ok result]
   - Match! Extract result, remove msg2

3. Return to process with bound 'result'

Mailbox after: [msg1] [msg3] [msg4]
               (msg2 was consumed)

Non-matching messages remain for future receives.
```

---

## Process Linking and Monitoring

Processes can be notified when other processes exit.

### Links (Bidirectional)

```
PROCESS LINKS
════════════════════════════════════════════════════════════════════════════════

(spawn-link (fn [] (worker-loop)))

Creates bidirectional link:

Process A ◄────link────► Process B

If A crashes:
  B receives exit signal (crashes too, unless trapping)

If B crashes:
  A receives exit signal (crashes too, unless trapping)

Use case: Supervisor trees, coordinated shutdown
```

### Monitors (Unidirectional)

```
PROCESS MONITORS
════════════════════════════════════════════════════════════════════════════════

(spawn-monitor (fn [] (worker-loop)))

Creates unidirectional monitor:

Process A ────monitors────► Process B

If B crashes:
  A receives [:DOWN ref pid reason] message

If A crashes:
  Nothing happens to B (unidirectional)

Use case: Watching without crashing together
```

### Exit Signals

```
EXIT SIGNAL PROPAGATION
════════════════════════════════════════════════════════════════════════════════

Process exits with reason:

Normal exit (:normal):
  - Links NOT notified (clean shutdown)
  - Monitors receive [:DOWN ref pid :normal]

Crash exit (:error, exception, etc.):
  - Links receive exit signal
  - Linked processes crash (unless trapping exits)
  - Monitors receive [:DOWN ref pid reason]

Trapping exits:
  (process-flag :trap-exit true)

  - Converts exit signals to messages
  - Process receives [:EXIT from-pid reason]
  - Used by supervisors to handle child crashes
```

---

## Differences from BEAM

Lonala follows BEAM's process model closely. The key differences are due to running on seL4 rather than a traditional OS:

| Aspect | BEAM/Erlang | Lonala |
|--------|-------------|--------|
| **Kernel** | OS process on Linux/Windows | seL4 microkernel (formally verified) |
| **Isolation boundary** | OS process | Realm (seL4 protection domain) |
| **Inter-node messaging** | Erlang distribution protocol | seL4 IPC + serialization |
| **CPU scheduling** | BEAM scheduler only | Three layers: seL4 MCS → workers → processes |
| **Memory protection** | None between processes | Realm boundaries enforced by hardware |
| **Hot code loading** | Full OTP support | Var rebinding (simpler, no code_change callbacks) |

The core process semantics are identical:
- Per-process heap with generational GC
- Message passing with deep copy
- Links and monitors
- Selective receive
- Reduction-based preemption

---

## Summary

| Aspect | Description |
|--------|-------------|
| **Process creation** | ~1-10 µs, pure userspace |
| **Initial heap size** | ~2 KB (configurable, enables millions of processes) |
| **Heap structure** | Two heaps: young (stack + objects) and old (promoted) |
| **Heap growth** | Fibonacci sequence then 20% increments |
| **Memory allocation** | Per-worker allocator instances, lock-free for common case |
| **GC model** | Generational copying, per-process, no global pauses |
| **Minor GC** | Promotes live young objects to old heap, ~10-100 µs pause |
| **Major GC** | Full sweep of both heaps, ~100 µs - few ms pause |
| **Large binaries** | Reference counted, stored in realm-wide binary heap |
| **Message passing** | Deep copy (heap fragments if contention), lock-free mailbox |
| **Intra-realm latency** | ~100-500 ns (deep copy) |
| **Inter-realm latency** | ~1-10 µs (serialization + IPC) |
| **Scheduling** | Reduction-based cooperative + MCS preemptive |
| **Fault isolation** | Crash affects only linked processes |
