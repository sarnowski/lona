// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Per-worker run queue for process scheduling.

use crate::process::ProcessId;

/// Maximum number of processes in a run queue.
pub const RUN_QUEUE_CAPACITY: usize = 256;

/// Per-worker FIFO queue storing `ProcessId`s.
pub struct RunQueue {
    buffer: [ProcessId; RUN_QUEUE_CAPACITY],
    head: usize,
    tail: usize,
    len: usize,
}

impl RunQueue {
    /// Create a new empty run queue.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: [ProcessId::NULL; RUN_QUEUE_CAPACITY],
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    /// Add process to back of queue.
    ///
    /// Returns `false` if queue is full.
    pub const fn push_back(&mut self, pid: ProcessId) -> bool {
        if self.len >= RUN_QUEUE_CAPACITY {
            return false;
        }

        self.buffer[self.tail] = pid;
        self.tail = (self.tail + 1) % RUN_QUEUE_CAPACITY;
        self.len += 1;
        true
    }

    /// Remove process from front of queue.
    ///
    /// Returns `None` if queue is empty.
    pub const fn pop_front(&mut self) -> Option<ProcessId> {
        if self.len == 0 {
            return None;
        }

        let pid = self.buffer[self.head];
        self.head = (self.head + 1) % RUN_QUEUE_CAPACITY;
        self.len -= 1;
        Some(pid)
    }

    /// Steal process from back of queue (for work stealing).
    ///
    /// Returns `None` if queue is empty.
    pub const fn steal_back(&mut self) -> Option<ProcessId> {
        if self.len == 0 {
            return None;
        }

        // Move tail back
        self.tail = if self.tail == 0 {
            RUN_QUEUE_CAPACITY - 1
        } else {
            self.tail - 1
        };

        let pid = self.buffer[self.tail];
        self.len -= 1;
        Some(pid)
    }

    /// Check if queue is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Number of processes in queue.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Check if queue is full.
    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.len >= RUN_QUEUE_CAPACITY
    }
}

impl Default for RunQueue {
    fn default() -> Self {
        Self::new()
    }
}
