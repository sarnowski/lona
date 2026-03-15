// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Spinlock-based reader-writer lock for `no_std` environments.
//!
//! Multiple readers can hold the lock concurrently. A writer waits for
//! all readers to release, then acquires exclusive access.
//!
//! State encoding in `AtomicUsize`:
//! - `0`: unlocked
//! - `1..usize::MAX-1`: number of active readers
//! - `usize::MAX`: write-locked

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicUsize, Ordering};

/// Sentinel value indicating the lock is write-locked.
const WRITE_LOCKED: usize = usize::MAX;

/// A spinlock-based reader-writer lock.
///
/// Allows multiple concurrent readers or one exclusive writer.
/// With seL4 MCS scheduling, a spinning TCB exhausts its budget
/// and is preempted, bounding starvation.
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
    /// writer currently holds the lock.
    pub fn read(&self) -> SpinReadGuard<'_, T> {
        loop {
            let state = self.state.load(Ordering::Relaxed);
            // Cannot acquire read lock if write-locked
            if state == WRITE_LOCKED {
                core::hint::spin_loop();
                continue;
            }
            // Reader count must never reach WRITE_LOCKED (bounded by MAX_WORKERS << usize::MAX)
            debug_assert!(state < WRITE_LOCKED - 1, "SpinRwLock reader count overflow");
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
    /// Waits for all readers to release, then acquires exclusive access.
    pub fn write(&self) -> SpinWriteGuard<'_, T> {
        loop {
            // Try to transition from unlocked (0) to write-locked
            if self
                .state
                .compare_exchange_weak(0, WRITE_LOCKED, Ordering::Acquire, Ordering::Relaxed)
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
