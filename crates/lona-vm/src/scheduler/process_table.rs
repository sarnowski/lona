// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Segmented process table with generation-based slot reuse.
//!
//! The process table uses two-level indexing for scalable process storage:
//! an array of segment pointers (L1) maps to segment arrays of Slots (L2).
//! Segments are allocated lazily from the `ProcessPool`, so no memory is used
//! until the first process is spawned.
//!
//! This design supports up to `MAX_SEGMENTS` × `SEGMENT_SIZE` concurrent
//! processes (1M with default settings), limited only by available memory
//! rather than a fixed constant.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::process::heap_fragment::HeapFragment;
use crate::process::{Process, ProcessId};
use crate::term::Term;

/// Number of slots per segment (must be a power of 2).
pub const SEGMENT_SIZE: usize = 1024;

/// Bit shift for segment indexing (log2 of `SEGMENT_SIZE`).
const SEGMENT_BITS: usize = 10;

/// Maximum number of segments (pointer array is inline, 8 bytes × 1024 = 8KB).
pub const MAX_SEGMENTS: usize = 1024;

const _: () = assert!(
    1usize << SEGMENT_BITS == SEGMENT_SIZE,
    "SEGMENT_BITS must equal log2(SEGMENT_SIZE)"
);
const _: () = assert!(
    SEGMENT_SIZE.is_power_of_two(),
    "SEGMENT_SIZE must be a power of two"
);

/// Slot in the process table.
pub struct Slot {
    /// The process in this slot, or None if free or taken.
    process: Option<Process>,
    /// Generation counter, incremented on each reuse.
    generation: u32,
    /// Index of next free slot (used when slot is free). `u32::MAX` = end of list.
    next_free: u32,
    /// Whether this slot is allocated (true from `allocate` until `remove`/`free_taken_slot`).
    allocated: bool,
    /// Inbox of heap fragments for messages sent while process was taken.
    ///
    /// When a process is taken (running on another worker), senders push
    /// fragments here instead of writing to the process's mailbox directly.
    /// The fragments are drained when the process is put back or during GC.
    fragment_inbox: Option<Box<HeapFragment>>,
    /// Pending exit signals for processes that are currently taken.
    ///
    /// When an exit signal targets a taken process (running on a worker),
    /// it is queued here. The signals are drained when the process is put
    /// back via `take_pending_signals`.
    pending_signals: Vec<(ProcessId, Term)>,
}

/// Segmented table for O(1) process lookup by PID.
///
/// Uses two-level indexing: `segments[index >> SEGMENT_BITS]` gives the
/// segment pointer, `index & (SEGMENT_SIZE - 1)` gives the slot offset.
/// Segments are allocated lazily — memory cost is zero until first spawn.
///
/// A free list chains slots across segments for O(1) allocation. Generation
/// counters prevent the ABA problem when slots are reused.
pub struct ProcessTable {
    /// Segment pointers (null = not allocated).
    segments: [*mut Slot; MAX_SEGMENTS],
    /// Number of allocated segments.
    num_segments: usize,
    /// Total capacity (`num_segments` × `SEGMENT_SIZE`).
    total_capacity: usize,
    /// Head of free list (flat index, or `u32::MAX` if empty).
    free_head: u32,
    /// Number of active processes.
    count: usize,
}

// SAFETY: Segment pointers point to exclusively-owned memory allocated
// via `Box::into_raw` (in tests) or `ProcessPool` (on seL4). The `ProcessTable`
// is always behind `SpinMutex`, preventing concurrent access.
unsafe impl Send for ProcessTable {}

/// Get raw pointer to a slot by flat index and segment array.
///
/// This is a free function (not a method) to avoid borrowing `self`,
/// enabling callers to modify other `ProcessTable` fields while holding
/// a raw pointer to a slot.
const fn slot_ptr(
    segments: &[*mut Slot; MAX_SEGMENTS],
    num_segments: usize,
    index: usize,
) -> Option<*mut Slot> {
    let segment_idx = index >> SEGMENT_BITS;
    let slot_idx = index & (SEGMENT_SIZE - 1);

    if segment_idx >= num_segments {
        return None;
    }

    let segment_ptr = segments[segment_idx];
    if segment_ptr.is_null() {
        return None;
    }

    // SAFETY: segment_ptr was initialized in grow_segment with SEGMENT_SIZE
    // slots, and slot_idx < SEGMENT_SIZE due to the bitmask.
    Some(unsafe { segment_ptr.add(slot_idx) })
}

impl ProcessTable {
    /// Create a new empty process table with no segments allocated.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            segments: [core::ptr::null_mut(); MAX_SEGMENTS],
            num_segments: 0,
            total_capacity: 0,
            free_head: u32::MAX,
            count: 0,
        }
    }

    /// Check if there are free slots available without allocating.
    #[must_use]
    pub const fn has_free_slots(&self) -> bool {
        self.free_head != u32::MAX
    }

    /// Add a pre-allocated segment to the table.
    ///
    /// The `segment_ptr` must point to a contiguous allocation of at least
    /// `SEGMENT_SIZE` `Slot`s. On seL4, this comes from `ProcessPool::allocate`.
    /// In tests, it comes from `Box::into_raw`.
    ///
    /// The new segment's slots are initialized and prepended to the free list.
    ///
    /// # Safety
    ///
    /// `segment_ptr` must point to valid, exclusively-owned memory large enough
    /// for `SEGMENT_SIZE` Slots. The caller transfers ownership to the table.
    ///
    /// # Panics
    ///
    /// Panics if the segment table is full (`num_segments >= MAX_SEGMENTS`).
    pub unsafe fn grow_segment(&mut self, segment_ptr: *mut Slot) {
        assert!(
            self.num_segments < MAX_SEGMENTS,
            "segment table full ({MAX_SEGMENTS} segments)"
        );

        let segment_idx = self.num_segments;
        let base_index = (segment_idx * SEGMENT_SIZE) as u32;

        // Initialize slots in the segment and chain them into the free list.
        // Chain from last to first so that the first slot in the segment
        // is at the head of the new free list chain.
        let mut chain_next = self.free_head;
        for i in (0..SEGMENT_SIZE).rev() {
            let sp = unsafe { segment_ptr.add(i) };
            unsafe {
                sp.write(Slot {
                    process: None,
                    generation: 0,
                    next_free: chain_next,
                    allocated: false,
                    fragment_inbox: None,
                    pending_signals: Vec::new(),
                });
            }
            chain_next = base_index + i as u32;
        }

        self.free_head = base_index;
        self.segments[segment_idx] = segment_ptr;
        self.num_segments += 1;
        self.total_capacity += SEGMENT_SIZE;
    }

    /// Allocate a slot, returns (index, generation) for creating `ProcessId`.
    ///
    /// Returns `None` if no free slots are available. Call `has_free_slots()`
    /// first, and use `grow_segment()` to add capacity if needed.
    pub fn allocate(&mut self) -> Option<(u32, u32)> {
        if self.free_head == u32::MAX {
            return None;
        }

        let index = self.free_head;
        let ptr = slot_ptr(&self.segments, self.num_segments, index as usize)?;
        // SAFETY: ptr is valid and we have exclusive access via &mut self.
        let slot = unsafe { &mut *ptr };

        self.free_head = slot.next_free;
        slot.allocated = true;

        Some((index, slot.generation))
    }

    /// Insert process into previously allocated slot.
    ///
    /// The `process.pid` must match the slot allocated by `allocate()`.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    pub fn insert(&mut self, process: Process) {
        let index = process.pid.index();
        let Some(ptr) = slot_ptr(&self.segments, self.num_segments, index) else {
            debug_assert!(false, "insert: index {index} out of bounds");
            return;
        };
        // SAFETY: ptr is valid and we have exclusive access via &mut self.
        let slot = unsafe { &mut *ptr };
        debug_assert!(slot.process.is_none(), "Slot already occupied");
        debug_assert_eq!(
            slot.generation,
            process.pid.generation(),
            "Generation mismatch"
        );

        slot.process = Some(process);
        self.count += 1;
    }

    /// Get process by PID (validates generation).
    #[must_use]
    pub fn get(&self, pid: ProcessId) -> Option<&Process> {
        if pid.is_null() {
            return None;
        }

        let ptr = slot_ptr(&self.segments, self.num_segments, pid.index())?;
        // SAFETY: ptr is valid and we have at least shared access via &self.
        let slot = unsafe { &*ptr };
        if slot.generation != pid.generation() {
            return None;
        }

        slot.process.as_ref()
    }

    /// Get mutable process by PID (validates generation).
    pub fn get_mut(&mut self, pid: ProcessId) -> Option<&mut Process> {
        if pid.is_null() {
            return None;
        }

        let ptr = slot_ptr(&self.segments, self.num_segments, pid.index())?;
        // SAFETY: ptr is valid and we have exclusive access via &mut self.
        let slot = unsafe { &mut *ptr };
        if slot.generation != pid.generation() {
            return None;
        }

        slot.process.as_mut()
    }

    /// Remove process from table, return slot to free list.
    ///
    /// Returns the removed process if PID was valid.
    pub fn remove(&mut self, pid: ProcessId) -> Option<Process> {
        if pid.is_null() {
            return None;
        }

        let index = pid.index();
        let ptr = slot_ptr(&self.segments, self.num_segments, index)?;
        // SAFETY: ptr is valid and we have exclusive access via &mut self.
        let slot = unsafe { &mut *ptr };
        if slot.generation != pid.generation() {
            return None;
        }

        let process = slot.process.take()?;

        slot.generation = slot.generation.wrapping_add(1);
        slot.allocated = false;
        slot.fragment_inbox = None;
        slot.pending_signals.clear();

        slot.next_free = self.free_head;
        self.free_head = index as u32;

        self.count -= 1;
        Some(process)
    }

    /// Temporarily extract a process for execution.
    ///
    /// The slot remains allocated (generation unchanged) but the process
    /// is removed. Use `put_back` to return it, or `free_taken_slot`
    /// to reclaim the slot after the process completes.
    pub fn take(&mut self, pid: ProcessId) -> Option<Process> {
        if pid.is_null() {
            return None;
        }

        let ptr = slot_ptr(&self.segments, self.num_segments, pid.index())?;
        // SAFETY: ptr is valid and we have exclusive access via &mut self.
        let slot = unsafe { &mut *ptr };
        if slot.generation != pid.generation() {
            return None;
        }

        let process = slot.process.take()?;
        // count stays the same — slot is still logically occupied
        Some(process)
    }

    /// Return a previously taken process to its slot.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    pub fn put_back(&mut self, pid: ProcessId, process: Process) {
        let index = pid.index();
        let Some(ptr) = slot_ptr(&self.segments, self.num_segments, index) else {
            debug_assert!(false, "put_back: index {index} out of bounds");
            return;
        };
        // SAFETY: ptr is valid and we have exclusive access via &mut self.
        let slot = unsafe { &mut *ptr };
        debug_assert!(
            slot.process.is_none(),
            "Slot already occupied (put_back on non-taken slot)"
        );
        debug_assert_eq!(
            slot.generation,
            pid.generation(),
            "Generation mismatch on put_back"
        );

        slot.process = Some(process);
    }

    /// Reclaim a slot after a taken process completes.
    ///
    /// Increments generation and returns the slot to the free list.
    /// Call this instead of `put_back` when the process is done.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    pub fn free_taken_slot(&mut self, pid: ProcessId) {
        let index = pid.index();
        let Some(ptr) = slot_ptr(&self.segments, self.num_segments, index) else {
            debug_assert!(false, "free_taken_slot: index {index} out of bounds");
            return;
        };
        // SAFETY: ptr is valid and we have exclusive access via &mut self.
        let slot = unsafe { &mut *ptr };
        debug_assert!(
            slot.process.is_none(),
            "Slot still has process (use remove instead)"
        );
        debug_assert_eq!(
            slot.generation,
            pid.generation(),
            "Generation mismatch on free_taken_slot"
        );

        slot.generation = slot.generation.wrapping_add(1);
        slot.allocated = false;
        slot.fragment_inbox = None;
        slot.pending_signals.clear();
        slot.next_free = self.free_head;
        self.free_head = index as u32;
        self.count -= 1;
    }

    /// Check if a slot is allocated but its process was taken.
    ///
    /// Returns `true` if the slot's generation matches, is allocated,
    /// and the process is currently extracted (e.g., being executed).
    #[must_use]
    pub fn is_taken(&self, pid: ProcessId) -> bool {
        if pid.is_null() {
            return false;
        }

        let Some(ptr) = slot_ptr(&self.segments, self.num_segments, pid.index()) else {
            return false;
        };
        // SAFETY: ptr is valid and we have at least shared access via &self.
        let slot = unsafe { &*ptr };

        slot.allocated && slot.generation == pid.generation() && slot.process.is_none()
    }

    /// Push a heap fragment to a process's inbox.
    ///
    /// Used for cross-worker message delivery when the process is taken.
    /// The fragment is prepended to the slot's fragment linked list (LIFO).
    ///
    /// **Important:** Prepend means fragments are in reverse send order.
    /// When draining the inbox into the process mailbox, the linked list
    /// must be reversed to maintain FIFO delivery order.
    ///
    /// Silently ignored if the PID is invalid or the slot is not allocated.
    pub fn push_fragment(&mut self, pid: ProcessId, mut fragment: Box<HeapFragment>) {
        if pid.is_null() {
            return;
        }
        let Some(ptr) = slot_ptr(&self.segments, self.num_segments, pid.index()) else {
            return;
        };
        // SAFETY: ptr is valid and we have exclusive access via &mut self.
        let slot = unsafe { &mut *ptr };
        if slot.generation != pid.generation() || !slot.allocated {
            return;
        }
        fragment.next = slot.fragment_inbox.take();
        slot.fragment_inbox = Some(fragment);
    }

    /// Take all fragments from a process's inbox.
    ///
    /// Returns the head of the fragment linked list, or `None` if empty.
    /// The slot's inbox is cleared.
    pub fn take_fragments(&mut self, pid: ProcessId) -> Option<Box<HeapFragment>> {
        if pid.is_null() {
            return None;
        }
        let ptr = slot_ptr(&self.segments, self.num_segments, pid.index())?;
        // SAFETY: ptr is valid and we have exclusive access via &mut self.
        let slot = unsafe { &mut *ptr };
        if slot.generation != pid.generation() {
            return None;
        }
        slot.fragment_inbox.take()
    }

    /// Queue an exit signal for a taken (running) process.
    ///
    /// When exit propagation targets a process that is currently taken
    /// for execution, the signal cannot be delivered immediately. It is
    /// queued here and drained when the process is put back.
    ///
    /// Silently ignored if the PID is invalid or the slot is not allocated.
    pub fn push_pending_signal(&mut self, pid: ProcessId, sender: ProcessId, reason: Term) {
        if pid.is_null() {
            return;
        }
        let Some(ptr) = slot_ptr(&self.segments, self.num_segments, pid.index()) else {
            return;
        };
        // SAFETY: ptr is valid and we have exclusive access via &mut self.
        let slot = unsafe { &mut *ptr };
        if slot.generation != pid.generation() || !slot.allocated {
            return;
        }
        slot.pending_signals.push((sender, reason));
    }

    /// Drain all pending exit signals for a process.
    ///
    /// Returns the signals (sender, reason) and clears the queue.
    /// Returns an empty `Vec` if no signals are pending.
    pub fn take_pending_signals(&mut self, pid: ProcessId) -> Vec<(ProcessId, Term)> {
        if pid.is_null() {
            return Vec::new();
        }
        let Some(ptr) = slot_ptr(&self.segments, self.num_segments, pid.index()) else {
            return Vec::new();
        };
        // SAFETY: ptr is valid and we have exclusive access via &mut self.
        let slot = unsafe { &mut *ptr };
        if slot.generation != pid.generation() {
            return Vec::new();
        }
        core::mem::take(&mut slot.pending_signals)
    }

    /// Number of active processes.
    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
    }

    /// Check if table has no free slots.
    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.free_head == u32::MAX
    }

    /// Number of allocated segments.
    #[must_use]
    pub const fn num_segments(&self) -> usize {
        self.num_segments
    }

    /// Total slot capacity across all segments.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.total_capacity
    }

    /// Allocate a test segment from the system heap.
    ///
    /// Returns a raw pointer to `SEGMENT_SIZE` heap-allocated Slots.
    /// The caller must pass this to `grow_segment` — the `ProcessTable`
    /// takes ownership and the memory must not be freed separately.
    #[cfg(test)]
    #[must_use]
    pub fn alloc_test_segment() -> *mut Slot {
        let slots: Vec<Slot> = (0..SEGMENT_SIZE)
            .map(|_| Slot {
                process: None,
                generation: 0,
                next_free: u32::MAX,
                allocated: false,
                fragment_inbox: None,
                pending_signals: Vec::new(),
            })
            .collect();
        let boxed: Box<[Slot]> = slots.into_boxed_slice();
        Box::into_raw(boxed).cast::<Slot>()
    }
}

impl Default for ProcessTable {
    fn default() -> Self {
        Self::new()
    }
}
