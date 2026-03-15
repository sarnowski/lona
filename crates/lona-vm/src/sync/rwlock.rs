// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Spinlock-based reader-writer lock for `no_std` environments.
//!
//! Multiple readers can hold the lock concurrently. A writer waits for
//! all readers to release, then acquires exclusive access.
//!
//! **Writer-preferring:** When a writer is waiting, new readers are blocked
//! until the writer acquires and releases the lock. This prevents writer
//! starvation when readers continuously enter and exit.
//!
//! State encoding in `AtomicUsize`:
//! - `0`: unlocked
//! - `1..WRITER_PENDING-1`: number of active readers
//! - Bit 62 set: a writer is waiting (blocks new readers)
//! - `usize::MAX`: write-locked

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicUsize, Ordering};

/// Sentinel value indicating the lock is write-locked.
const WRITE_LOCKED: usize = usize::MAX;

/// Bit flag indicating a writer is waiting. Set in the state to block
/// new readers from acquiring the lock until the writer gets in.
const WRITER_PENDING: usize = 1 << 62;

/// Mask for the reader count (lower 62 bits).
const READER_MASK: usize = WRITER_PENDING - 1;

/// A spinlock-based reader-writer lock.
///
/// Allows multiple concurrent readers or one exclusive writer.
/// Writer-preferring: when a writer is waiting, new readers spin until
/// the writer finishes. With seL4 MCS scheduling, a spinning TCB exhausts
/// its budget and is preempted, bounding starvation.
pub struct SpinRwLock<T> {
    state: AtomicUsize,
    data: UnsafeCell<T>,
}

// SAFETY: SpinRwLock provides synchronized access. Shared read access
// requires T: Sync; exclusive write access requires T: Send.
unsafe impl<T: Send> Send for SpinRwLock<T> {}
unsafe impl<T: Send + Sync> Sync for SpinRwLock<T> {}

impl<T> SpinRwLock<T> {
    /// Create a new unlocked reader-writer lock.
    #[must_use]
    pub const fn new(data: T) -> Self {
        Self {
            state: AtomicUsize::new(0),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquire a read lock, spinning until available.
    ///
    /// Multiple readers can hold the lock concurrently. Spins if a
    /// writer currently holds or is waiting for the lock.
    pub fn read(&self) -> SpinReadGuard<'_, T> {
        loop {
            let state = self.state.load(Ordering::Relaxed);
            // Cannot acquire read lock if write-locked or writer pending
            if state == WRITE_LOCKED || state & WRITER_PENDING != 0 {
                core::hint::spin_loop();
                continue;
            }
            let readers = state & READER_MASK;
            debug_assert!(readers < READER_MASK, "SpinRwLock reader count overflow");
            // Try to increment reader count
            if self
                .state
                .compare_exchange_weak(state, state + 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return SpinReadGuard { lock: self };
            }
        }
    }

    /// Acquire a write lock, spinning until available.
    ///
    /// Sets a writer-pending flag to block new readers, then waits for
    /// all existing readers to release before acquiring exclusive access.
    pub fn write(&self) -> SpinWriteGuard<'_, T> {
        // Phase 1: set writer-pending to block new readers.
        // Loop until we successfully set the WRITER_PENDING bit.
        loop {
            let state = self.state.load(Ordering::Relaxed);
            if state == WRITE_LOCKED {
                // Another writer holds the lock
                core::hint::spin_loop();
                continue;
            }
            if state & WRITER_PENDING != 0 {
                // Another writer is already pending
                core::hint::spin_loop();
                continue;
            }
            // Set writer-pending flag
            if self
                .state
                .compare_exchange_weak(
                    state,
                    state | WRITER_PENDING,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                break;
            }
        }

        // Phase 2: wait for all readers to drain, then acquire.
        loop {
            // Try to transition from WRITER_PENDING (no readers) to WRITE_LOCKED
            if self
                .state
                .compare_exchange_weak(
                    WRITER_PENDING,
                    WRITE_LOCKED,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return SpinWriteGuard { lock: self };
            }
            core::hint::spin_loop();
        }
    }

    /// Consume the lock and return the inner data.
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }

    /// Get a mutable reference to the inner data.
    ///
    /// Since this takes `&mut self`, no locking is needed — the caller
    /// has exclusive access by the borrow checker.
    pub const fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }
}

/// RAII guard for a shared read lock on a `SpinRwLock`.
pub struct SpinReadGuard<'a, T> {
    lock: &'a SpinRwLock<T>,
}

impl<T> Deref for SpinReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: Read guard ensures no writer holds the lock.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> Drop for SpinReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.fetch_sub(1, Ordering::Release);
    }
}

/// RAII guard for an exclusive write lock on a `SpinRwLock`.
pub struct SpinWriteGuard<'a, T> {
    lock: &'a SpinRwLock<T>,
}

impl<T> Deref for SpinWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: Write guard ensures exclusive access.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: Write guard ensures exclusive access.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.store(0, Ordering::Release);
    }
}
