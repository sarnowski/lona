// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the binary buffer type.

use super::*;

#[test]
fn creation_produces_owned_zeroed_buffer() {
    let buf = Binary::new(10_usize);
    assert!(buf.is_owner());
    assert!(!buf.is_zombie());
    assert_eq!(buf.len(), 10_usize);

    // Verify all bytes are zero
    for i in 0_usize..10_usize {
        assert_eq!(buf.get(i).unwrap(), Some(0_u8));
    }
}

#[test]
fn from_vec_preserves_data() {
    let data = alloc::vec![1_u8, 2_u8, 3_u8, 4_u8, 5_u8];
    let buf = Binary::from_vec(data);
    assert!(buf.is_owner());
    assert_eq!(buf.len(), 5_usize);
    assert_eq!(buf.get(0_usize).unwrap(), Some(1_u8));
    assert_eq!(buf.get(4_usize).unwrap(), Some(5_u8));
}

#[test]
fn len_and_is_empty() {
    let buf = Binary::new(10_usize);
    assert_eq!(buf.len(), 10_usize);
    assert!(!buf.is_empty());

    let empty = Binary::new(0_usize);
    assert_eq!(empty.len(), 0_usize);
    assert!(empty.is_empty());
}

#[test]
fn is_owner_returns_correct_access_mode() {
    let owned = Binary::new(10_usize);
    assert!(owned.is_owner());

    let view = owned.view();
    assert!(!view.is_owner());
}

#[test]
fn get_and_set_with_owned_buffer() {
    let buf = Binary::new(10_usize);

    // Set some values
    assert!(buf.set(0_usize, 42_u8).is_ok());
    assert!(buf.set(9_usize, 99_u8).is_ok());

    // Read them back
    assert_eq!(buf.get(0_usize).unwrap(), Some(42_u8));
    assert_eq!(buf.get(9_usize).unwrap(), Some(99_u8));
}

#[test]
fn get_with_view_works() {
    let owned = Binary::new(10_usize);
    owned.set(5_usize, 123_u8).unwrap();

    let view = owned.view();
    assert_eq!(view.get(5_usize).unwrap(), Some(123_u8));
}

#[test]
fn set_with_view_returns_read_only_error() {
    let owned = Binary::new(10_usize);
    let view = owned.view();

    assert_eq!(view.set(0_usize, 42_u8), Err(Error::ReadOnly));
}

#[test]
fn get_out_of_bounds_returns_none() {
    let buf = Binary::new(10_usize);
    assert_eq!(buf.get(10_usize).unwrap(), None);
    assert_eq!(buf.get(100_usize).unwrap(), None);
}

#[test]
fn set_out_of_bounds_returns_error() {
    let buf = Binary::new(10_usize);
    assert_eq!(buf.set(10_usize, 42_u8), Err(Error::OutOfBounds));
    assert_eq!(buf.set(100_usize, 42_u8), Err(Error::OutOfBounds));
}

#[test]
fn slice_of_owned_returns_owned() {
    let owned = Binary::new(100_usize);
    owned.set(50_usize, 42_u8).unwrap();

    let slice = owned.slice(25_usize, 50_usize).unwrap();
    assert!(slice.is_owner());
    assert_eq!(slice.len(), 50_usize);

    // Index 50 in original is index 25 in slice
    assert_eq!(slice.get(25_usize).unwrap(), Some(42_u8));

    // Slice can still write (it's owned)
    assert!(slice.set(0_usize, 99_u8).is_ok());
}

#[test]
fn slice_of_view_returns_view() {
    let owned = Binary::new(100_usize);
    let view = owned.view();

    let slice = view.slice(25_usize, 50_usize).unwrap();
    assert!(!slice.is_owner());
    assert_eq!(slice.len(), 50_usize);
}

#[test]
fn slice_bounds_checking() {
    let buf = Binary::new(100_usize);

    // Valid slice
    assert!(buf.slice(0_usize, 100_usize).is_some());
    assert!(buf.slice(50_usize, 50_usize).is_some());
    assert!(buf.slice(99_usize, 1_usize).is_some());
    assert!(buf.slice(100_usize, 0_usize).is_some());

    // Invalid slices
    assert!(buf.slice(0_usize, 101_usize).is_none());
    assert!(buf.slice(50_usize, 51_usize).is_none());
    assert!(buf.slice(101_usize, 0_usize).is_none());
}

#[test]
fn view_always_returns_view() {
    let owned = Binary::new(10_usize);
    let view1 = owned.view();
    let view2 = view1.view();

    assert!(!view1.is_owner());
    assert!(!view2.is_owner());
}

#[test]
fn clone_always_produces_view() {
    let owned = Binary::new(10_usize);
    let cloned = owned.clone();

    assert!(owned.is_owner());
    assert!(!cloned.is_owner());

    // Clone of a view is also a view
    let view = owned.view();
    let view_clone = view.clone();
    assert!(!view_clone.is_owner());
}

#[test]
fn equality_based_on_content() {
    let buf1 = Binary::from_vec(alloc::vec![1_u8, 2_u8, 3_u8]);
    let buf2 = Binary::from_vec(alloc::vec![1_u8, 2_u8, 3_u8]);
    let buf3 = Binary::from_vec(alloc::vec![1_u8, 2_u8, 4_u8]);

    assert_eq!(buf1, buf2);
    assert_ne!(buf1, buf3);
}

#[test]
fn equality_ignores_access_mode() {
    let owned = Binary::from_vec(alloc::vec![1_u8, 2_u8, 3_u8]);
    let view = owned.view();

    assert_eq!(owned, view);
}

#[test]
fn display_format() {
    let owned = Binary::new(1024_usize);
    assert_eq!(alloc::format!("{owned}"), "#<binary:1024 owned>");

    let view = owned.view();
    assert_eq!(alloc::format!("{view}"), "#<binary:1024 view>");
}

#[test]
fn display_empty_buffer() {
    let empty = Binary::new(0_usize);
    assert_eq!(alloc::format!("{empty}"), "#<binary:0 owned>");
}

#[test]
fn as_bytes_returns_correct_slice() {
    let buf = Binary::from_vec(alloc::vec![1_u8, 2_u8, 3_u8, 4_u8, 5_u8]);
    let bytes = buf.as_bytes().unwrap();
    assert_eq!(bytes.as_slice(), &[1_u8, 2_u8, 3_u8, 4_u8, 5_u8]);
}

#[test]
fn as_bytes_respects_slice_bounds() {
    let buf = Binary::from_vec(alloc::vec![1_u8, 2_u8, 3_u8, 4_u8, 5_u8]);
    let slice = buf.slice(1_usize, 3_usize).unwrap();
    let bytes = slice.as_bytes().unwrap();
    assert_eq!(bytes.as_slice(), &[2_u8, 3_u8, 4_u8]);
}

#[test]
fn views_share_underlying_data() {
    let owned = Binary::new(10_usize);
    let view = owned.view();

    // Modify through owned
    owned.set(5_usize, 42_u8).unwrap();

    // Change is visible through view
    assert_eq!(view.get(5_usize).unwrap(), Some(42_u8));
}

#[test]
fn slices_share_underlying_data() {
    let owned = Binary::new(100_usize);
    let slice = owned.slice(25_usize, 50_usize).unwrap();

    // Modify through slice (it's also owned)
    slice.set(0_usize, 99_u8).unwrap();

    // Change is visible in original at offset 25
    assert_eq!(owned.get(25_usize).unwrap(), Some(99_u8));
}

#[test]
fn phys_addr_operations() {
    let owned = Binary::new(10_usize);
    assert_eq!(owned.phys_addr(), None);

    // Set physical address
    owned.set_phys_addr(0x1000_u64).unwrap();
    assert_eq!(owned.phys_addr(), Some(0x1000_u64));

    // View can read but not write
    let view = owned.view();
    assert_eq!(view.phys_addr(), Some(0x1000_u64));
    assert_eq!(view.set_phys_addr(0x2000_u64), Err(Error::ReadOnly));
}
