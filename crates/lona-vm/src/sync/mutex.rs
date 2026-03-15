// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Spinlock-based mutex for `no_std` environments.
//!
//! Uses `AtomicBool::compare_exchange` with `Acquire`/`Release` ordering.
//! The spin loop uses `core::hint::spin_loop()` for CPU-friendly spinning.

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

/// A spinlock-based mutual exclusion primitive.
///
/// Provides exclusive access to the wrapped data. When contended, the
/// acquiring thread spins until the lock becomes available. With seL4
/// MCS scheduling, a spinning TCB exhausts its budget and is preempted,
/// bounding starvation.
pub struct SpinMutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

// SAFETY: SpinMutex provides synchronized access. The data is only
// accessible through the guard, which enforces exclusive access.
unsafe impl<T: Send> Send for SpinMutex<T> {}
unsafe impl<T: Send> Sync for SpinMutex<T> {}

impl<T> SpinMutex<T> {
    /// Create a new unlocked mutex wrapping the given data.
    #[must_use]
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquire the lock, spinning until it becomes available.
    ///
    /// Returns an RAII guard that releases the lock when dropped.
    pub fn lock(&self) -> SpinMutexGuard<'_, T> {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Spin-wait: hint to the CPU that we're in a busy-wait loop.
            // On ARM, this emits YIELD; on x86, this emits PAUSE.
            while self.locked.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
        }
        SpinMutexGuard { mutex: self }
    }

    /// Try to acquire the lock without spinning.
    ///
    /// Returns `Some(guard)` if the lock was acquired, `None` if it
    /// is already held.
    pub fn try_lock(&self) -> Option<SpinMutexGuard<'_, T>> {
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(SpinMutexGuard { mutex: self })
        } else {
            None
        }
    }

    /// Consume the mutex and return the inner data.
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

/// RAII guard that releases the `SpinMutex` when dropped.
pub struct SpinMutexGuard<'a, T> {
    mutex: &'a SpinMutex<T>,
}

impl<T> Deref for SpinMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: The guard guarantees exclusive access to the data.
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T> DerefMut for SpinMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: The guard guarantees exclusive access to the data.
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T> Drop for SpinMutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::Release);
    }
}
