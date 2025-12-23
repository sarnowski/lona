# Process Communication & Memory Sharing

This document defines the architecture for inter-process and inter-domain communication in Lona. We adopt **pure BEAM semantics**: all messages are deep-copied except for large binaries, which use reference-counted sharing.

---

## Table of Contents

1. [Design Principles](#design-principles)
2. [The Lona Memory Model](#the-lona-memory-model)
3. [Binary: The Large Data Escape Hatch](#binary-the-large-data-escape-hatch)
4. [Technical Specifications](#technical-specifications)
5. [Data Flow Analysis](#data-flow-analysis)
6. [Cross-Domain Reference Counting Protocol](#cross-domain-reference-counting-protocol)
7. [Process Exit and Resource Cleanup](#process-exit-and-resource-cleanup)
8. [Crash Handling](#crash-handling)
9. [Ring Buffer Protocol](#ring-buffer-protocol)
10. [Network Stack Example](#network-stack-example)
11. [Bindings, Closures, and Environment](#bindings-closures-and-environment)
12. [Implementation Phases](#implementation-phases)
13. [Summary](#summary)

---

## Design Principles

### Why Pure BEAM Semantics?

We adopt BEAM's copy-on-send model because it provides:

| Property | Benefit |
|----------|---------|
| **Per-process heap isolation** | Each process GCs independently; one process's GC doesn't affect others |
| **Immediate memory release** | When a process dies, its heap is instantly reclaimable |
| **No shared mutable state** | Eliminates data races by design |
| **Simple mental model** | Messages are always independent copies |

The alternative—sharing immutable references across processes—would require domain-wide tracing GC and break BEAM's isolation guarantees.

### The Binary Exception

BEAM makes one exception: **large binaries** are reference-counted and shared. This is the proven pattern for efficient large data handling without sacrificing isolation.

Lona follows this with **threshold-based sharing**:

| Context | Threshold | Behavior |
|---------|-----------|----------|
| **Intra-domain** | > 64 bytes | Share by reference (atomic refcount is cheap) |
| **Cross-domain** | > 4 KB | Share via capability-gated shared memory |
| **Cross-domain** | ≤ 4 KB | Inline copy in seL4 IPC (avoids capability/mapping overhead) |

**Rationale**: Intra-domain sharing has minimal overhead (atomic increment). Cross-domain sharing requires capability grants, VSpace mappings, and IPC-based refcounting—this overhead only pays off for larger data.

---

## The Lona Memory Model

### Message Passing Semantics

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         MESSAGE PASSING RULES                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  INTRA-DOMAIN (Process A → Process B, same domain):                         │
│  ──────────────────────────────────────────────────                         │
│    • Maps, vectors, lists, etc.: DEEP COPY                                  │
│    • Binary (> 64 bytes): Share reference (receiver gets read-only view)    │
│    • Binary (≤ 64 bytes): DEEP COPY                                         │
│    • Atoms: CANNOT cross process boundary                                   │
│                                                                             │
│  INTER-DOMAIN (Domain A → Domain B):                                        │
│  ────────────────────────────────────                                       │
│    • Maps, vectors, lists, etc.: DEEP COPY via serialization                │
│    • Binary (> 4 KB): Share via capability-gated shared memory              │
│    • Binary (≤ 4 KB): Inline copy in seL4 IPC                               │
│    • Atoms: CANNOT cross domain boundary                                    │
│    • Closures with captures: ERROR                                          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Why Copy Even Intra-Domain?

This differs from the "share immutable references" approach. The rationale:

1. **BEAM compatibility**: Erlang copies all messages for good reason
2. **Per-process GC**: Sharing references couples process heaps, breaking independent GC
3. **Crash isolation**: Dead process memory can be immediately freed
4. **Predictable latency**: No domain-wide GC pauses

For large data, use `Binary` explicitly—this is the escape hatch.

---

## Binary: The Large Data Escape Hatch

### Binary Ownership Model

```clojure
;; Create an owned binary (can read and write)
(def buf (binary-create 1024))
(binary-set! buf 0 0xFF)

;; Create a read-only view (slice)
(def header (binary-view buf 0 64))

;; Views cannot be written to
(binary-set! header 0 0x00)  ; ERROR: view is read-only
```

### Binary Modes

| Mode | Can Read | Can Write | Created By |
|------|----------|-----------|------------|
| `Owned` | Yes | Yes | `binary-create`, `binary-create-dma` |
| `View` | Yes | No | `binary-view`, receiving from another process |

### Binary Sharing Thresholds

| Context | Threshold | Action |
|---------|-----------|--------|
| **Intra-domain send** | > 64 bytes | Share reference, increment refcount |
| **Intra-domain send** | ≤ 64 bytes | Deep copy bytes to receiver's heap |
| **Cross-domain send** | > 4 KB | Create SharedRegion, grant capability, send BinaryRef |
| **Cross-domain send** | ≤ 4 KB | Serialize bytes inline in seL4 IPC message |

**Rationale for split thresholds**:
- Intra-domain: Atomic refcount increment is ~1 CPU cycle. Worth it even for 65 bytes.
- Cross-domain: Capability grant + VSpace mapping + IPC refcount protocol is ~1000s of cycles. Only worth it for larger data.

---

## Technical Specifications

This section provides the concrete Rust data structures required for implementation.

### Binary Value Representation

```rust
/// A Binary handle that lives on a process heap
pub struct Binary {
    /// Pointer to the backing storage (may be shared)
    backing: BinaryBacking,
    /// Offset into the buffer (for views/slices)
    offset: u32,
    /// Length of this view
    len: u32,
    /// Access mode (Owned can write, View is read-only)
    mode: BinaryMode,
}

pub enum BinaryMode {
    /// Original creator - can read and write
    Owned,
    /// Derived from another Binary or received via message - read only
    View,
}

pub enum BinaryBacking {
    /// Intra-domain: refcounted byte buffer in domain memory
    Local {
        /// Pointer to BinaryData header + bytes
        data: NonNull<BinaryData>,
    },
    /// Cross-domain: reference to shared memory region
    Shared {
        /// The shared region this binary belongs to
        region_id: RegionId,
        /// Base offset within the region
        base_offset: u32,
    },
}
```

### Binary Backing Storage (Off-Heap)

```rust
/// Off-heap storage for binary data (not on any process heap)
/// Allocated from domain's binary heap
#[repr(C)]
pub struct BinaryData {
    /// Reference count (atomic for intra-domain sharing)
    refcount: AtomicU32,
    /// Generation counter for ABA protection
    generation: u32,
    /// Capacity of the buffer
    capacity: u32,
    /// Actual length of valid data
    len: u32,
    /// Flexible array member: actual bytes follow this header
    bytes: [u8; 0],
}

impl BinaryData {
    /// Size of header before bytes
    pub const HEADER_SIZE: usize = 16;
}
```

### BinaryKey for Cross-Domain Identification

To prevent ABA bugs when buffers are recycled, every cross-domain binary reference includes a generation counter:

```rust
/// Unique identifier for a binary across domain boundaries
/// Used in retain/release IPC messages
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BinaryKey {
    /// Which shared region contains this binary
    pub region_id: u32,
    /// Offset within the region
    pub offset: u32,
    /// Length of the binary
    pub len: u32,
    /// Generation counter - incremented on each allocation
    /// Prevents ABA: late Release for old allocation won't affect new one
    pub generation: u32,
}
```

**ABA Protection**: When a buffer is freed and the same offset is reused for a new allocation, the generation counter increments. Late-arriving Release IPCs for the old generation are ignored because their generation doesn't match.

### BinaryRef (Cross-Domain Handle)

```rust
/// Serializable handle sent across domain boundaries
/// Does NOT contain a pointer - contains identification info
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BinaryRef {
    /// Unique key identifying this binary
    pub key: BinaryKey,
    /// Access mode for receiver (always View for received refs)
    pub mode: BinaryMode,
    /// Domain that owns the backing storage
    pub owner_domain: DomainId,
}
```

### Shared Memory Region

```rust
/// A shared memory region that can be mapped into multiple domains
pub struct SharedRegion {
    /// Unique identifier
    pub id: RegionId,
    /// seL4 frame capabilities for the physical pages
    pub frame_caps: Vec<seL4_CPtr>,
    /// Total size in bytes
    pub size: usize,
    /// Domain that owns this region (responsible for allocation/deallocation)
    pub owner: DomainId,
    /// Per-domain mapping information
    pub mappings: BTreeMap<DomainId, RegionMapping>,
    /// Sub-allocator for binaries within this region
    pub allocator: RegionAllocator,
}

pub struct RegionMapping {
    /// Virtual address where mapped in that domain
    pub vaddr: usize,
    /// Access rights granted to this domain
    pub rights: RegionRights,
    /// seL4 capability for the mapping (for revocation)
    pub mapping_cap: seL4_CPtr,
}

pub enum RegionRights {
    ReadOnly,
    ReadWrite,
}
```

### Owner-Authoritative Reference Tracking

```rust
/// Tracks references to binaries owned by this domain
/// Lives in the owner domain's memory
pub struct OwnerRefTable {
    /// Map from BinaryKey to tracking entry
    entries: BTreeMap<BinaryKey, RefEntry>,
}

pub struct RefEntry {
    /// Local references (processes in owner domain)
    pub local_refs: u32,
    /// Per-remote-domain reference counts
    /// Needed for crash recovery: if domain X crashes, we clear its refs
    pub remote_refs: BTreeMap<DomainId, u32>,
    /// Pointer to the actual BinaryData (for freeing)
    pub data: NonNull<BinaryData>,
}

impl RefEntry {
    pub fn total_refs(&self) -> u32 {
        self.local_refs + self.remote_refs.values().sum::<u32>()
    }
}
```

---

## Data Flow Analysis

### Intra-Domain Message Send (Process A → Process B)

```
1. Process A executes: (send pid-B {:key value :data some-binary})

2. Runtime performs send-copy traversal:

   FOR EACH element in message:

   2a. Immutable values (maps, vectors, keywords, numbers, strings):
       → Allocate equivalent structure in B's heap
       → Deep copy all nested values recursively
       → A's original values unchanged

   2b. Binary values (size > 64 bytes):
       → Increment refcount on BinaryData (atomic)
       → Create new Binary handle in B's heap with:
          - Same backing pointer
          - Same offset/len
          - mode = View (regardless of A's mode)
       → NO byte copy occurs

   2c. Binary values (size ≤ 64 bytes):
       → Allocate new BinaryData in B's domain binary heap
       → Copy bytes from A's binary to new allocation
       → Create Binary handle pointing to new BinaryData

   2d. Atoms:
       → ERROR: Cannot send atoms across process boundary

3. Enqueue copied message in Process B's mailbox

4. Process A continues (send is asynchronous)

5. Later, when B's GC collects the Binary:
   → Decrement refcount (atomic)
   → If refcount reaches 0, free BinaryData
```

### Cross-Domain Message Send (Domain A → Domain B)

```
1. Process in Domain A executes: (send pid-in-B {:data some-binary})

2. Runtime detects cross-domain send (target pid's domain ≠ current)

3. FOR EACH element in message:

   3a. Immutable values:
       → Serialize to compact binary format
       → Place in seL4 IPC message buffer

   3b. Binary values (size > 4 KB):

       3b.1. PRE-RETAIN PROTOCOL (CRITICAL FOR CORRECTNESS):
             IF sender domain ≠ owner domain:
               → Send Retain IPC to owner: { key, from: B, count: 1 }
               → WAIT for acknowledgment
               → If owner is dead, abort send with error
             ELSE (sender is owner):
               → Increment remote_refs[B] in OwnerRefTable

       3b.2. Ensure B has capability to the SharedRegion:
             IF B not in region.mappings:
               → Grant B read-only frame capability
               → B maps region into its VSpace

       3b.3. Construct BinaryRef:
             { key: { region, offset, len, generation },
               mode: View,
               owner_domain: A }

       3b.4. Include BinaryRef in IPC message

   3c. Binary values (size ≤ 4 KB):
       → Serialize bytes inline in IPC message
       → Receiver will allocate fresh BinaryData

   3d. Closures with captures:
       → ERROR: Cannot send capturing closures cross-domain

4. Perform seL4 IPC (seL4_Call or seL4_Send)

5. Domain B receives and deserializes:
   → Allocate process-heap structures for immutable data
   → For BinaryRef: create Binary handle pointing to mapped region
   → For inline bytes: allocate new BinaryData, copy bytes

6. When B's process drops the Binary:
   → Queue Release IPC to owner domain (batched)
```

---

## Cross-Domain Reference Counting Protocol

### Protocol Overview

Since domains have separate address spaces, we cannot use shared atomic counters. The **owner-authoritative** model uses IPC:

1. **Owner domain** maintains all refcount state in its own memory
2. **Retain**: Before sending BinaryRef to domain B, increment B's count
3. **Release**: When B's GC drops the ref, send Release IPC to owner
4. **Batching**: Accumulate retain/release ops and flush periodically

### Retain/Release Message Format

```rust
/// IPC message types for refcount management
pub enum RefcountOp {
    /// Increment refcount (sent before exposing BinaryRef to receiver)
    Retain {
        key: BinaryKey,
        from_domain: DomainId,
        count: u32,
    },
    /// Decrement refcount (sent when GC drops reference)
    Release {
        key: BinaryKey,
        from_domain: DomainId,
        count: u32,
    },
}

/// Batched refcount operations for efficiency
pub struct RefcountBatch {
    pub from_domain: DomainId,
    pub ops: Vec<(RefcountOp, BinaryKey, u32)>,
}
```

### Pre-Retain Protocol (Critical for Correctness)

**Problem**: If sender sends BinaryRef before Retain reaches owner, owner might free the buffer.

**Solution**: The retain MUST complete before the message containing BinaryRef is sent.

```
SEND PATH (cross-domain Binary > 4KB):

1. Sender prepares message, finds Binary to share

2. IF sender ≠ owner:
   a. Send Retain IPC to owner domain
   b. BLOCK until Retain acknowledged
   c. If Retain fails (owner dead): abort send with {:error :owner-died}

3. IF sender = owner:
   a. Directly increment remote_refs[receiver] in OwnerRefTable

4. NOW safe to include BinaryRef in message and send

5. Receiver receives message, creates View handle
```

### Batching Strategy

To reduce IPC overhead, retain/release operations are batched:

```rust
/// Per-domain outgoing batch buffer
pub struct RefcountBatchBuffer {
    /// Target domain for these ops
    target: DomainId,
    /// Accumulated operations
    ops: Vec<(RefcountOp, BinaryKey, u32)>,
    /// Timestamp of first op (for timeout-based flush)
    first_op_time: Option<Instant>,
}

/// Flush conditions (any triggers flush):
/// - Buffer reaches 64 operations
/// - 10ms since first operation
/// - Process about to block (in receive)
/// - Domain shutdown
```

**Note**: Retain operations for cross-domain sends are NOT batched—they must complete synchronously before the send. Only Release operations (from GC) are batched.

### Handling Stale Operations (ABA Protection)

When owner receives a Release for a BinaryKey:

```rust
fn handle_release(key: BinaryKey, from: DomainId, count: u32) {
    if let Some(entry) = self.ref_table.get_mut(&key) {
        // Check generation matches
        if entry.data.generation != key.generation {
            // Stale release for recycled slot - ignore
            return;
        }

        // Decrement count for this domain
        if let Some(remote_count) = entry.remote_refs.get_mut(&from) {
            *remote_count = remote_count.saturating_sub(count);
            if *remote_count == 0 {
                entry.remote_refs.remove(&from);
            }
        }

        // Check if fully released
        if entry.total_refs() == 0 {
            // Free the binary data
            self.free_binary(entry.data);
            self.ref_table.remove(&key);
        }
    }
    // Key not found - already freed, ignore
}
```

---

## Process Exit and Resource Cleanup

### The Problem

BEAM achieves "instant heap reclaim" by bulk-freeing process heaps without running destructors. If Binary uses Rust's `Drop` for refcount management, bulk freeing leaks refcounts.

### Solution: Off-Heap Resource List

Each process maintains a list of external resources (binaries, capabilities, etc.) separate from its heap:

```rust
/// External resources owned by a process
/// Stored outside the process heap for explicit cleanup
pub struct ProcessResources {
    /// Binary references this process holds
    /// Each entry is (BinaryBacking, needs_release: bool)
    pub binary_refs: Vec<BinaryResource>,

    /// Other external resources...
    pub capabilities: Vec<seL4_CPtr>,
}

pub struct BinaryResource {
    /// The backing storage reference
    pub backing: BinaryBacking,
    /// True if this is a cross-domain ref needing Release IPC
    pub needs_release_ipc: bool,
    /// Owner domain (for Release IPC)
    pub owner_domain: Option<DomainId>,
    /// Key (for Release IPC)
    pub key: Option<BinaryKey>,
}
```

### Process Exit Sequence

```rust
fn process_exit(process: &mut Process, exit_reason: ExitReason) {
    // 1. Walk resource list and clean up binaries
    for resource in process.resources.binary_refs.drain(..) {
        match resource.backing {
            BinaryBacking::Local { data } => {
                // Decrement local refcount
                let old = unsafe { (*data.as_ptr()).refcount.fetch_sub(1, Release) };
                if old == 1 {
                    // Last reference - free the BinaryData
                    free_binary_data(data);
                }
            }
            BinaryBacking::Shared { region_id, .. } => {
                if resource.needs_release_ipc {
                    // Queue Release IPC to owner domain
                    let key = resource.key.unwrap();
                    let owner = resource.owner_domain.unwrap();
                    queue_release(owner, key, 1);
                }
            }
        }
    }

    // 2. Flush any pending Release IPCs
    flush_release_batches();

    // 3. Release capabilities
    for cap in process.resources.capabilities.drain(..) {
        seL4_CNode_Delete(cap);
    }

    // 4. NOW safe to bulk-free the process heap
    process.heap.bulk_free();

    // 5. Notify linked processes, supervisor, etc.
    notify_exit(process.pid, exit_reason);
}
```

### Binary Handle Lifecycle

When a Binary is created or received:

```rust
fn register_binary(process: &mut Process, binary: &Binary) {
    let resource = BinaryResource {
        backing: binary.backing.clone(),
        needs_release_ipc: matches!(binary.backing, BinaryBacking::Shared { .. }),
        owner_domain: binary.owner_domain(),
        key: binary.key(),
    };
    process.resources.binary_refs.push(resource);
}
```

The Binary handle on the process heap does NOT have a `Drop` impl—cleanup happens via the resource list on process exit.

---

## Crash Handling

### Domain Crash Detection

When a domain crashes (detected by seL4 or supervisor):

```clojure
;; Supervisor broadcasts to all domains:
{:op :domain-down :domain-id crashed-domain-id}
```

### Owner Domain Crashes

When the **owner** of shared binaries crashes:

1. **seL4 revokes frame capabilities** for all SharedRegions owned by crashed domain
2. **Other domains** holding Views will page fault on access
3. **Page fault handler** converts fault to Lonala exception

```rust
/// Page fault handler for shared memory access
fn handle_page_fault(fault_addr: usize, process: &mut Process) -> FaultResult {
    // Check if fault is in a shared region
    if let Some(region) = find_region_for_addr(fault_addr) {
        // Check if owner domain is alive
        if !is_domain_alive(region.owner) {
            // Owner crashed - inject exception into process
            return FaultResult::InjectException(
                LonalaError::OwnerDied {
                    region_id: region.id,
                    owner_domain: region.owner,
                }
            );
        }
    }

    // Not a shared region or other error - crash process
    FaultResult::CrashProcess
}
```

The Lonala code can catch this exception:

```clojure
(try
  (binary-get shared-data 0)
  (catch {:error :owner-died}
    (log "Owner domain crashed, handling gracefully...")
    (use-fallback-data)))
```

### Holder Domain Crashes

When a domain holding **Views** (not owner) crashes:

1. **Owner domain** receives `:domain-down` broadcast
2. **Owner clears** all remote_refs for crashed domain
3. **Owner frees** binaries if total refcount reaches 0

```rust
fn handle_domain_down(crashed: DomainId) {
    for (key, entry) in self.ref_table.iter_mut() {
        // Remove all refs attributed to crashed domain
        if entry.remote_refs.remove(&crashed).is_some() {
            // Check if fully released
            if entry.total_refs() == 0 {
                self.pending_free.push(*key);
            }
        }
    }

    // Free all fully-released binaries
    for key in self.pending_free.drain(..) {
        if let Some(entry) = self.ref_table.remove(&key) {
            self.free_binary(entry.data);
        }
    }
}
```

---

## Ring Buffer Protocol

For high-throughput data paths (NIC ↔ TCP), we use shared memory ring buffers.

### Ring Buffer Structure

```rust
/// Single-Producer Single-Consumer ring buffer
/// Used for high-throughput inter-domain communication
#[repr(C)]
pub struct RingBuffer {
    /// Metadata header (cache-line aligned)
    pub meta: RingMeta,
    /// Descriptor array follows
    pub descriptors: [RingDescriptor; 0],  // Flexible array
}

/// Ring metadata with cache-line separation to prevent false sharing
#[repr(C)]
pub struct RingMeta {
    // === Cache line 1: Producer writes, consumer reads ===
    /// Next slot to write (producer increments after write)
    pub head: AtomicU32,
    /// Padding to fill cache line
    _pad1: [u8; 60],

    // === Cache line 2: Consumer writes, producer reads ===
    /// Next slot to read (consumer increments after read)
    pub tail: AtomicU32,
    /// Padding to fill cache line
    _pad2: [u8; 60],

    // === Cache line 3: Read-only configuration ===
    /// Number of slots (must be power of 2)
    pub size: u32,
    /// seL4 notification capability for signaling
    pub notify_cap: seL4_CPtr,
    /// Policy when ring is full
    pub full_policy: RingFullPolicy,
    /// Padding
    _pad3: [u8; 48],
}

/// What to do when ring is full
#[repr(u32)]
pub enum RingFullPolicy {
    /// Drop new entries when full (default for network RX)
    TailDrop = 0,
    /// Overwrite oldest unconsumed entries (for logging)
    Overwrite = 1,
    /// Block producer until space available (for control messages)
    Block = 2,
    /// Signal backpressure to producer (let producer decide)
    Backpressure = 3,
}

/// Single ring entry describing a buffer
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RingDescriptor {
    /// ID of buffer in shared data region
    pub buffer_id: u32,
    /// Offset within that buffer
    pub offset: u32,
    /// Length of valid data
    pub len: u32,
    /// Flags (e.g., MORE_FRAGMENTS, CHECKSUM_VALID)
    pub flags: u32,
}
```

### Ring Operations

```rust
impl RingBuffer {
    /// Check if ring is full
    pub fn is_full(&self) -> bool {
        let head = self.meta.head.load(Acquire);
        let tail = self.meta.tail.load(Acquire);
        (head.wrapping_sub(tail)) >= self.meta.size
    }

    /// Check if ring is empty
    pub fn is_empty(&self) -> bool {
        let head = self.meta.head.load(Acquire);
        let tail = self.meta.tail.load(Acquire);
        head == tail
    }

    /// Producer: write descriptor and advance head
    pub fn produce(&self, desc: RingDescriptor) -> Result<(), RingFullError> {
        if self.is_full() {
            return Err(RingFullError);
        }

        let head = self.meta.head.load(Relaxed);
        let idx = (head as usize) & (self.meta.size as usize - 1);

        // Write descriptor
        unsafe {
            let desc_ptr = self.descriptors.as_ptr().add(idx) as *mut RingDescriptor;
            desc_ptr.write(desc);
        }

        // Memory barrier: ensure descriptor written before head update
        fence(Release);

        // Advance head
        self.meta.head.store(head.wrapping_add(1), Release);

        Ok(())
    }

    /// Consumer: read descriptor and advance tail
    pub fn consume(&self) -> Option<RingDescriptor> {
        if self.is_empty() {
            return None;
        }

        let tail = self.meta.tail.load(Relaxed);

        // Memory barrier: ensure we see descriptor writes
        fence(Acquire);

        let idx = (tail as usize) & (self.meta.size as usize - 1);

        // Read descriptor
        let desc = unsafe {
            let desc_ptr = self.descriptors.as_ptr().add(idx);
            desc_ptr.read()
        };

        // Advance tail
        self.meta.tail.store(tail.wrapping_add(1), Release);

        Some(desc)
    }
}
```

### Memory Ordering (ARM)

On ARM and other weakly-ordered architectures:

| Operation | Barrier | Rust Atomic |
|-----------|---------|-------------|
| Producer: write desc | `dmb st` | `fence(Release)` |
| Producer: write head | Release store | `store(_, Release)` |
| Consumer: read head | Acquire load | `load(Acquire)` |
| Consumer: read desc | `dmb ld` | `fence(Acquire)` |
| Consumer: write tail | Release store | `store(_, Release)` |

### Ring Full Policy Behavior

| Policy | Behavior | Use Case |
|--------|----------|----------|
| `TailDrop` | `produce()` returns error, caller drops | Network RX (TCP retransmits) |
| `Overwrite` | Oldest unconsumed entry overwritten | Logging, metrics |
| `Block` | Producer blocks until space | Control messages |
| `Backpressure` | Signal sent to producer | Flow control |

---

## Network Stack Example

This example demonstrates how Binary sharing works across three isolated domains.

### Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         NETWORK STACK DOMAINS                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐         │
│  │ Domain: NIC     │    │ Domain: TCP     │    │ Domain: Telnet  │         │
│  │                 │    │                 │    │                 │         │
│  │ - DMA buffers   │    │ - IP routing    │    │ - REPL session  │         │
│  │ - IRQ handling  │    │ - TCP state     │    │ - User input    │         │
│  │ - Ring buffers  │    │ - Connections   │    │ - Per-connection│         │
│  └────────┬────────┘    └────────┬────────┘    └────────┬────────┘         │
│           │                      │                      │                   │
│           │    Shared RX         │    Per-Connection    │                   │
│           │    Region (RO)       │    Stream Region     │                   │
│           └──────────────────────┴──────────────────────┘                   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Memory Regions

| Region | Owner | NIC Access | TCP Access | Telnet Access |
|--------|-------|------------|------------|---------------|
| RX Data Region (DMA) | NIC | Read-Write | Read-Only | None |
| RX Descriptor Ring | NIC | Read-Write | Read-Only | None |
| Per-Connection Stream | TCP | None | Read-Write | Read-Only (own conn only) |

### Packet Flow with Copy Points

```
STEP 1: Hardware DMA
════════════════════
[Network Wire] ──DMA──▶ [RX Data Region, buffer_id=42]

Copy: NONE (hardware DMA)


STEP 2: NIC → TCP (Descriptor)
══════════════════════════════
NIC writes to ring: { buffer_id: 42, offset: 0, len: 1500, flags: 0 }
NIC signals: seL4_Signal(rx_notify)
TCP wakes, reads descriptor from ring

Copy: ~16 bytes (descriptor only, NOT packet data)


STEP 3: TCP Parses Headers
══════════════════════════
TCP reads directly from RX Data Region (mapped read-only)
Parses: Ethernet → IP → TCP headers
Looks up connection in connection table

Copy: NONE (zero-copy header inspection)


STEP 4: TCP → Telnet (Threshold-Based)
══════════════════════════════════════

Small payload (≤ 4 KB, typical for interactive):
──────────────────────────────────────────────────
TCP extracts payload, copies inline in IPC message

┌──────────────────┐         ┌──────────────────┐
│ RX Data Region   │ ──COPY──▶│ Telnet process   │
│ [hdrs][PAYLOAD]  │  payload │ heap Binary      │
└──────────────────┘  only    └──────────────────┘

Copy: ~100 bytes (typical Telnet command)


Large payload (> 4 KB, file transfer):
──────────────────────────────────────
TCP writes payload to per-connection stream region
Sends BinaryRef handle to Telnet

┌──────────────────┐         ┌──────────────────┐
│ Conn Stream      │         │ Telnet process   │
│ Region [conn_0]  │◀────────│ BinaryRef (View) │
└──────────────────┘ shared  └──────────────────┘

Copy: NONE (shared region + BinaryRef)


STEP 5: Buffer Return
═════════════════════
TCP sends: { :op :rx-release :buffer-ids [42] }
NIC recycles buffer for next DMA
```

### Copy Summary

| Path | Copies | What's Copied |
|------|--------|---------------|
| Wire → NIC (DMA) | 0 | Hardware writes directly |
| NIC → TCP (descriptor) | 0 | Ring entry only (~16 bytes) |
| TCP header parse | 0 | Read-only mapping |
| TCP → Telnet (≤ 4 KB) | **1** | Payload only (not headers) |
| TCP → Telnet (> 4 KB) | 0 | Shared region + BinaryRef |

### Per-Connection Security Isolation

**Problem**: TCP handles ALL connections. Telnet for connection A must NOT see connection B's packets.

**Solution**: Capability-badged endpoints

```clojure
;; When Telnet accepts a connection:
;;
;; 1. TCP mints a badged capability
(def conn-cap (cap-mint conn-endpoint {:badge conn-id :rights [:send :recv]}))

;; 2. TCP grants to Telnet
(send telnet-domain {:op :accept :conn-cap conn-cap :peer peer-info})

;; 3. Telnet can ONLY use this connection
;;    - Cannot forge conn-id (kernel enforces badge)
;;    - Cannot access other connections' data
;;    - Cannot see RX Data Region (no capability)
```

### Capability Distribution

```
[Root/Init Domain]
       │
       │ creates all frames, endpoints, notifications
       ▼
[Net-Supervisor Domain]
       │
       ├──▶ [NIC Driver Domain]
       │      Granted:
       │        - Device MMIO cap (RW)
       │        - IRQ cap
       │        - rx_data_region cap (RW, for DMA)
       │        - rx_desc_ring cap (RW)
       │        - Endpoint to TCP (send)
       │
       └──▶ [TCP/IP Stack Domain]
              Granted:
                - rx_data_region cap (RO)  ◄── READ-ONLY
                - rx_desc_ring cap (RO)
                - Endpoint from NIC (recv)
              Creates per-connection:
                - conn_stream_region[conn_id]
                - ConnCap(badge=conn_id)
                     │
                     ▼
              [Telnet Server Domain]
                Granted per-connection:
                  - ConnCap(badge=conn_id)
                  - conn_stream_region[conn_id] cap (RO)
                NOT granted:
                  - rx_data_region (NIC buffers)
                  - Other connections' regions
                  - TCP's internal state
```

---

## Bindings, Closures, and Environment

### Principle: Explicit Data Flow

When spawning across domain boundaries, **nothing is implicitly inherited**.

```clojure
;; WRONG - closure captures secret
(def secret "password")
(spawn (fn [] (use secret)) [] {:domain "other"})
;; ERROR: Cannot spawn closure with captures across domain boundary

;; CORRECT - explicit argument passing
(defn worker [config]
  ;; All data is explicit
  (do-work config))

(spawn worker [{:host "localhost"}] {:domain "other"})
```

### Cross-Domain Closure Rules

| Closure Type | Allowed? | Rationale |
|--------------|----------|-----------|
| Zero captures | Yes | Nothing to leak |
| Captures immutable values | Error | Could leak secrets |
| Captures atoms | Error | Atoms are process-local |
| Captures capabilities | Error | Caps are domain-specific |

### Atom Semantics

Atoms are **strictly process-local**:

| Boundary | Behavior |
|----------|----------|
| Same process | Normal atom operations |
| Different process (same domain) | ERROR - cannot share atoms |
| Different domain | ERROR - cannot cross |

**Pattern**: Use a GenServer process to manage shared state:

```clojure
;; Instead of:
(def shared (atom {}))

;; Use:
(defn state-server [initial]
  (loop [state initial]
    (receive
      {:get :from pid}
        (do (send pid {:state state})
            (recur state))
      {:update :fn f}
        (recur (f state)))))
```

### Dynamic Variables

Dynamic bindings are **NOT inherited** across domain boundaries:

```clojure
(def ^:dynamic *user* nil)

(binding [*user* "alice"]
  ;; Within same domain: *user* is "alice"
  (spawn local-worker [])

  ;; Across domain boundary: *user* resets to nil
  (spawn 'worker/main [] {:domain "other"}))
```

**Rationale**: A new domain is a new context. Implicit context propagation is a security risk.

### Environment Inheritance Summary

| What | Intra-Domain Spawn | Inter-Domain Spawn |
|------|-------------------|-------------------|
| Function args | Deep copied | Deep copied |
| Global defs | Accessible (same namespace) | Copied via code loading |
| Dynamic vars | Inherited (binding stack) | Reset to defaults |
| Atoms | ERROR | ERROR |
| Closures with captures | Allowed | ERROR |
| Capabilities | Shared (same CSpace) | Must explicitly grant |

---

## Implementation Phases

### Phase 1: MVP (Milestone 1.6)

**All boundaries use copy semantics:**

1. **Intra-domain**: Deep copy all values (BEAM semantics)
2. **Inter-domain**: Serialize/deserialize via seL4 IPC
3. **Binary**: Process-local only (no cross-process sharing yet)
4. **Closures**: Error if captures + cross-domain spawn

This gives us correct semantics and enables development to proceed.

### Phase 2: Binary Sharing (Later)

**Add refc binary support:**

1. **Intra-domain Binary**: Share reference if > 64 bytes, copy if ≤ 64 bytes
2. **Inter-domain Binary**: Shared memory regions if > 4 KB, inline copy if ≤ 4 KB
3. **Refcount protocol**: Owner-authoritative with IPC retain/release
4. **Crash handling**: DOMAIN_DOWN broadcasts, page fault → exception
5. **Off-heap resource list**: For correct process exit semantics

### Phase 3: High-Throughput Optimization (Later)

**Add ring buffer infrastructure:**

1. **Ring buffer primitives**: Create, produce, consume
2. **Notification integration**: seL4_Signal/Wait
3. **Configurable full policy**: TailDrop, Overwrite, Block, Backpressure
4. **Per-connection regions**: For app-level zero-copy
5. **Memory barriers**: ARM barrier handling in runtime

### Phase 4: Advanced Features (Much Later)

1. **Binary transfer**: Move ownership instead of sharing
2. **Content-addressed store**: Optional deduplication
3. **Lazy serialization**: Only serialize on demand

---

## Summary

### The Lona Memory Model

| Aspect | Design Choice | Rationale |
|--------|---------------|-----------|
| Regular values | Deep copy on send | BEAM semantics, per-process GC |
| Binary (intra, > 64B) | Share reference | Atomic refcount is cheap |
| Binary (intra, ≤ 64B) | Deep copy | Avoid refcount overhead for tiny data |
| Binary (cross, > 4KB) | Shared memory + caps | Worth the setup overhead |
| Binary (cross, ≤ 4KB) | Inline copy | Avoids capability/mapping overhead |
| Closures cross-domain | Error if captures | Security, explicit data flow |
| Atoms | Process-local only | No shared mutable state |
| Refcounting | Owner-authoritative | No shared counters across domains |
| ABA protection | Generation counter | Prevent stale Release corruption |
| Process exit | Off-heap resource list | BEAM-style instant heap reclaim |
| Crash handling | Page fault → exception | Graceful error handling |

### Technical Decisions Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Intra-domain immutable values | Deep copy | Pure BEAM semantics, independent GC |
| Binary threshold (intra) | 64 bytes | BEAM compatibility, atomic refcount cheap |
| Binary threshold (cross) | 4 KB | Amortize capability/mapping overhead |
| ABA protection | Generation counter in BinaryKey | Simple, 4 bytes, standard solution |
| Retain/release race | Pre-retain before send | Correctness over performance |
| Process exit | Off-heap resource list | Enables bulk heap free |
| Crashed domain access | Page fault → exception | Graceful, catchable error |
| Ring buffer full | Configurable per ring | Different use cases need different policies |
| Ring cache layout | Separate cache lines | Prevent false sharing |

### Copy Points in Network Path

| Boundary | Copies | Notes |
|----------|--------|-------|
| DMA → NIC | 0 | Hardware direct |
| NIC → TCP | 0 | Shared ring + region |
| TCP → App (≤ 4 KB) | 1 | Inline copy, simple |
| TCP → App (> 4 KB) | 0 | Per-connection region |

---

## Further Reading

- [Core Concepts](../goals/core-concepts.md) — Domain, Process, Capability definitions
- [System Design](../goals/system-design.md) — Memory model details
- [Pillar: seL4](../goals/pillar-sel4.md) — Capability-based security
- [Pillar: BEAM](../goals/pillar-beam.md) — Message passing philosophy
- [Pillar: Clojure](../goals/pillar-clojure.md) — Immutable data structures
- [Domain Isolation Tasks](../roadmap/milestone-01-rust-foundation/06-domain-isolation.md) — Implementation tasks

## References

- [BEAM Book](https://blog.stenmans.org/theBeamBook/) — Erlang runtime internals
- [Erlang Efficiency Guide](https://www.erlang.org/doc/system/eff_guide_processes.html) — Process and binary handling
- [seL4 Device Driver Framework](https://trustworthy.systems/projects/drivers) — Ring buffer patterns
- [seL4 IPC Guide](https://microkerneldude.org/2019/03/07/how-to-and-how-not-to-use-sel4-ipc/) — IPC best practices
