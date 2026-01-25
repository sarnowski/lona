// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Process table with generation-based slot reuse.
//!
//! The process table is a fixed-size array that stores processes by slot index.
//! Each slot has a generation counter to prevent the ABA problem when slots
//! are reused after a process terminates.

extern crate alloc;

use crate::process::{Process, ProcessId};
use alloc::boxed::Box;
use alloc::vec::Vec;

/// Maximum number of processes in a realm.
pub const MAX_PROCESSES: usize = 1024;

/// Slot in the process table.
struct Slot {
    /// The process in this slot, or None if free.
    process: Option<Process>,
    /// Generation counter, incremented on each reuse.
    generation: u32,
    /// Index of next free slot (used when slot is free).
    next_free: u32,
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

        // Add to free list
        slot.next_free = self.free_head;
        self.free_head = index as u32;

        self.count -= 1;
        Some(process)
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
