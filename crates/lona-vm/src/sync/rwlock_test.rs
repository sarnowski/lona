// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for `SpinRwLock`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn rwlock_read_access() {
    let lock = SpinRwLock::new(42);
    let guard = lock.read();
    assert_eq!(*guard, 42);
}

#[test]
fn rwlock_write_access() {
    let lock = SpinRwLock::new(0);
    {
        let mut guard = lock.write();
        *guard = 99;
    }
    let guard = lock.read();
    assert_eq!(*guard, 99);
}

#[test]
fn rwlock_multiple_readers() {
    let lock = SpinRwLock::new(42);
    // Multiple read guards can coexist
    let guard1 = lock.read();
    let guard2 = lock.read();
    let guard3 = lock.read();
    assert_eq!(*guard1, 42);
    assert_eq!(*guard2, 42);
    assert_eq!(*guard3, 42);
}

#[test]
fn rwlock_write_after_read_release() {
    let lock = SpinRwLock::new(0);
    {
        let _reader = lock.read();
        // Reader holds lock
    }
    // Reader released, writer can proceed
    let mut writer = lock.write();
    *writer = 77;
    drop(writer);

    let reader = lock.read();
    assert_eq!(*reader, 77);
}

#[test]
fn rwlock_read_after_write_release() {
    let lock = SpinRwLock::new(0);
    {
        let mut writer = lock.write();
        *writer = 55;
    }
    // Writer released, readers can proceed
    let reader1 = lock.read();
    let reader2 = lock.read();
    assert_eq!(*reader1, 55);
    assert_eq!(*reader2, 55);
}

#[test]
fn rwlock_into_inner() {
    let lock = SpinRwLock::new(vec![1, 2, 3]);
    let data = lock.into_inner();
    assert_eq!(data, vec![1, 2, 3]);
}

#[test]
fn rwlock_get_mut() {
    let mut lock = SpinRwLock::new(0);
    *lock.get_mut() = 42;
    let guard = lock.read();
    assert_eq!(*guard, 42);
}

#[test]
fn rwlock_send_sync_bounds() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<SpinRwLock<u32>>();
    assert_sync::<SpinRwLock<u32>>();
    assert_send::<SpinRwLock<Vec<u8>>>();
    assert_sync::<SpinRwLock<Vec<u8>>>();
}

#[test]
fn rwlock_sequential_write_cycles() {
    let lock = SpinRwLock::new(0u32);
    for i in 0..100 {
        let mut writer = lock.write();
        *writer = i;
        drop(writer);
    }
    let reader = lock.read();
    assert_eq!(*reader, 99);
}

#[test]
fn rwlock_reader_deref() {
    let lock = SpinRwLock::new(vec![1, 2, 3]);
    let guard = lock.read();
    // Deref: can call Vec methods through the guard
    assert_eq!(guard.len(), 3);
    assert_eq!(guard[0], 1);
}

#[test]
fn rwlock_writer_deref_and_deref_mut() {
    let lock = SpinRwLock::new(vec![1, 2, 3]);
    {
        let mut guard = lock.write();
        // Deref: read access
        assert_eq!(guard.len(), 3);
        // DerefMut: write access
        guard.push(4);
    }
    let guard = lock.read();
    assert_eq!(*guard, vec![1, 2, 3, 4]);
}
