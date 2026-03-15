// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for `SpinMutex`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn mutex_lock_and_access() {
    let mutex = SpinMutex::new(42);
    let guard = mutex.lock();
    assert_eq!(*guard, 42);
}

#[test]
fn mutex_lock_mutate() {
    let mutex = SpinMutex::new(0);
    {
        let mut guard = mutex.lock();
        *guard = 99;
    }
    let guard = mutex.lock();
    assert_eq!(*guard, 99);
}

#[test]
fn mutex_try_lock_succeeds_when_unlocked() {
    let mutex = SpinMutex::new(10);
    let guard = mutex.try_lock();
    assert!(guard.is_some());
    assert_eq!(*guard.unwrap(), 10);
}

#[test]
fn mutex_try_lock_fails_when_locked() {
    let mutex = SpinMutex::new(10);
    let _guard = mutex.lock();
    assert!(mutex.try_lock().is_none());
}

#[test]
fn mutex_guard_auto_releases() {
    let mutex = SpinMutex::new(0);
    {
        let _guard = mutex.lock();
        // Guard is dropped here
    }
    // Lock should be available again
    let guard = mutex.try_lock();
    assert!(guard.is_some());
}

#[test]
fn mutex_into_inner() {
    let mutex = SpinMutex::new(vec![1, 2, 3]);
    let data = mutex.into_inner();
    assert_eq!(data, vec![1, 2, 3]);
}

#[test]
fn mutex_send_sync_bounds() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<SpinMutex<u32>>();
    assert_sync::<SpinMutex<u32>>();
    assert_send::<SpinMutex<Vec<u8>>>();
    assert_sync::<SpinMutex<Vec<u8>>>();
}

#[test]
fn mutex_sequential_lock_unlock_cycles() {
    let mutex = SpinMutex::new(0u32);
    for i in 0..100 {
        let mut guard = mutex.lock();
        *guard = i;
        drop(guard);
    }
    let guard = mutex.lock();
    assert_eq!(*guard, 99);
}

#[test]
fn mutex_deref_and_deref_mut() {
    let mutex = SpinMutex::new(vec![1, 2, 3]);
    {
        let mut guard = mutex.lock();
        // Deref: read access
        assert_eq!(guard.len(), 3);
        // DerefMut: write access
        guard.push(4);
    }
    let guard = mutex.lock();
    assert_eq!(*guard, vec![1, 2, 3, 4]);
}
