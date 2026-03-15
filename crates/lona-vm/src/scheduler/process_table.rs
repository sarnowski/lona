// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Process table with generation-based slot reuse.
//!
//! The process table is a fixed-size array that stores processes by slot index.
//! Each slot has a generation counter to prevent the ABA problem when slots
//! are reused after a process terminates.

extern crate alloc;

use crate::process::heap_fragment::HeapFragment;
use crate::process::{Process, ProcessId};
use alloc::boxed::Box;
use alloc::vec::Vec;

/// Maximum number of processes in a realm.
pub const MAX_PROCESSES: usize = 1024;

/// Slot in the process table.
struct Slot {
    /// The process in this slot, or None if free or taken.
    process: Option<Process>,
    /// Generation counter, incremented on each reuse.
    generation: u32,
    /// Index of next free slot (used when slot is free).
    next_free: u32,
    /// Whether this slot is allocated (true from `allocate` until `remove`/`free_taken_slot`).
    allocated: bool,
    /// Inbox of heap fragments for messages sent while process was taken.
    ///
    /// When a process is taken (running on another worker), senders push
    /// fragments here instead of writing to the process's mailbox directly.
    /// The fragments are drained when the process is put back or during GC.
    fragment_inbox: Option<Box<HeapFragment>>,
}

/// Fixed-size table for O(1) process lookup by PID.
///
/// Uses a free list for allocation and generation counters for ABA safety.
/// The slots are heap-allocated to keep the table off the stack.
pub struct ProcessTable {
    slots: Box<[Slot]>,
    /// Head of free list (index of first free slot, or `u32::MAX` if full).
    free_head: u32,
    /// Number of active processes.
    count: usize,
}

impl ProcessTable {
    /// Create a new empty process table.
    #[must_use]
    pub fn new() -> Self {
        // Initialize slots with free list chain
        // Use Vec to allocate directly on heap, keeping table off the stack
        let slots: Vec<Slot> = (0..MAX_PROCESSES)
            .map(|i| Slot {
                process: None,
                generation: 0,
                next_free: if i + 1 < MAX_PROCESSES {
                    (i + 1) as u32
                } else {
                    u32::MAX // End of free list
                },
                allocated: false,
                fragment_inbox: None,
            })
            .collect();

        Self {
            slots: slots.into_boxed_slice(),
            free_head: 0,
            count: 0,
        }
    }

    /// Allocate a slot, returns (index, generation) for creating `ProcessId`.
    ///
    /// Returns `None` if table is full.
    pub const fn allocate(&mut self) -> Option<(u32, u32)> {
        if self.free_head == u32::MAX {
            return None; // Table full
        }

        let index = self.free_head;
        let slot = &mut self.slots[index as usize];

        // Remove from free list
        self.free_head = slot.next_free;
        slot.allocated = true;

        // Return current generation (will be stored in ProcessId)
        Some((index, slot.generation))
    }

    /// Insert process into previously allocated slot.
    ///
    /// The `process.pid` must match the slot allocated by `allocate()`.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the PID doesn't match an allocated slot.
    pub fn insert(&mut self, process: Process) {
        let index = process.pid.index();
        debug_assert!(index < MAX_PROCESSES, "PID index out of bounds");
        debug_assert!(self.slots[index].process.is_none(), "Slot already occupied");
        debug_assert_eq!(
            self.slots[index].generation,
            process.pid.generation(),
            "Generation mismatch"
        );

        self.slots[index].process = Some(process);
        self.count += 1;
    }

    /// Get process by PID (validates generation).
    #[must_use]
    pub const fn get(&self, pid: ProcessId) -> Option<&Process> {
        if pid.is_null() {
            return None;
        }

        let index = pid.index();
        if index >= MAX_PROCESSES {
            return None;
        }

        let slot = &self.slots[index];
        if slot.generation != pid.generation() {
            return None; // Stale reference
        }

        slot.process.as_ref()
    }

    /// Get mutable process by PID (validates generation).
    pub const fn get_mut(&mut self, pid: ProcessId) -> Option<&mut Process> {
        if pid.is_null() {
            return None;
        }

        let index = pid.index();
        if index >= MAX_PROCESSES {
            return None;
        }

        let slot = &mut self.slots[index];
        if slot.generation != pid.generation() {
            return None; // Stale reference
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
        if index >= MAX_PROCESSES {
            return None;
        }

        let slot = &mut self.slots[index];
        if slot.generation != pid.generation() {
            return None; // Stale reference
        }

        let process = slot.process.take()?;

        // Increment generation for next reuse
        slot.generation = slot.generation.wrapping_add(1);
        slot.allocated = false;
        // Drop any pending fragments to prevent leaking into reused slots
        slot.fragment_inbox = None;

        // Add to free list
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
    ///
    /// This enables passing `&mut ProcessTable` to `Vm::run` while a
    /// process is being executed (no aliased borrows).
    pub fn take(&mut self, pid: ProcessId) -> Option<Process> {
        if pid.is_null() {
            return None;
        }

        let index = pid.index();
        if index >= MAX_PROCESSES {
            return None;
        }

        let slot = &mut self.slots[index];
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
    /// Panics in debug builds if:
    /// - The PID doesn't match the slot's generation
    /// - The slot already contains a process
    pub fn put_back(&mut self, pid: ProcessId, process: Process) {
        let index = pid.index();
        debug_assert!(index < MAX_PROCESSES, "PID index out of bounds");
        debug_assert!(
            self.slots[index].process.is_none(),
            "Slot already occupied (put_back on non-taken slot)"
        );
        debug_assert_eq!(
            self.slots[index].generation,
            pid.generation(),
            "Generation mismatch on put_back"
        );

        self.slots[index].process = Some(process);
    }

    /// Reclaim a slot after a taken process completes.
    ///
    /// Increments generation and returns the slot to the free list.
    /// Call this instead of `put_back` when the process is done.
    pub fn free_taken_slot(&mut self, pid: ProcessId) {
        let index = pid.index();
        debug_assert!(index < MAX_PROCESSES, "PID index out of bounds");
        debug_assert!(
            self.slots[index].process.is_none(),
            "Slot still has process (use remove instead)"
        );
        debug_assert_eq!(
            self.slots[index].generation,
            pid.generation(),
            "Generation mismatch on free_taken_slot"
        );

        let slot = &mut self.slots[index];
        slot.generation = slot.generation.wrapping_add(1);
        slot.allocated = false;
        // Drop any pending fragments to prevent leaking into reused slots
        slot.fragment_inbox = None;
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

        let index = pid.index();
        if index >= MAX_PROCESSES {
            return false;
        }

        let slot = &self.slots[index];
        slot.allocated && slot.generation == pid.generation() && slot.process.is_none()
    }

    /// Push a heap fragment to a process's inbox.
    ///
    /// Used for cross-worker message delivery when the process is taken.
    /// The fragment is prepended to the slot's fragment linked list (LIFO).
    ///
    /// **Important:** Prepend means fragments are in reverse send order.
    /// When draining the inbox into the process mailbox, the linked list
    /// must be reversed to maintain FIFO delivery order (spec guarantee:
    /// "order preserved between same sender-receiver pair").
    ///
    /// Silently ignored if the PID is invalid or the slot is not allocated.
    pub fn push_fragment(&mut self, pid: ProcessId, mut fragment: Box<HeapFragment>) {
        if pid.is_null() {
            return;
        }
        let index = pid.index();
        if index >= MAX_PROCESSES {
            return;
        }
        let slot = &mut self.slots[index];
        if slot.generation != pid.generation() || !slot.allocated {
            return;
        }
        // Prepend to linked list
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
        let index = pid.index();
        if index >= MAX_PROCESSES {
            return None;
        }
        let slot = &mut self.slots[index];
        if slot.generation != pid.generation() {
            return None;
        }
        slot.fragment_inbox.take()
    }

    /// Number of active processes.
    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
    }

    /// Check if table is full.
    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.free_head == u32::MAX
    }
}

impl Default for ProcessTable {
    fn default() -> Self {
        Self::new()
    }
}
