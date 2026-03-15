// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Synchronization primitives for multi-worker concurrency.
//!
//! seL4 provides no userspace mutex. These spinlock-based implementations
//! use `core::sync::atomic` for `no_std` compatibility. With MCS scheduling,
//! a spinning TCB exhausts its budget and is preempted, so starvation
//! is bounded.

mod mutex;
mod rwlock;

#[cfg(test)]
mod mutex_test;
#[cfg(test)]
mod rwlock_test;

pub use mutex::{SpinMutex, SpinMutexGuard};
pub use rwlock::{SpinReadGuard, SpinRwLock, SpinWriteGuard};
