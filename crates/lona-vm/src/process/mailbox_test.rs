// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the per-process mailbox.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::mailbox::Mailbox;
use crate::term::Term;

/// Helper: create a small integer Term (panics if out of range).
fn int(n: i64) -> Term {
    Term::small_int(n).unwrap()
}

#[test]
fn new_mailbox_is_empty() {
    let mbox = Mailbox::new();
    assert!(mbox.is_empty());
    assert_eq!(mbox.len(), 0);
    assert!(mbox.peek_from_save().is_none());
}

#[test]
fn push_and_len() {
    let mut mbox = Mailbox::new();
    mbox.push(int(1));
    mbox.push(int(2));
    mbox.push(int(3));
    assert_eq!(mbox.len(), 3);
    assert!(!mbox.is_empty());
}

#[test]
fn peek_returns_first_message() {
    let mut mbox = Mailbox::new();
    mbox.push(int(42));
    mbox.push(int(99));

    let (idx, msg) = mbox.peek_from_save().unwrap();
    assert_eq!(idx, 0);
    assert_eq!(msg, int(42));
}

#[test]
fn advance_save_skips_message() {
    let mut mbox = Mailbox::new();
    mbox.push(int(1));
    mbox.push(int(2));
    mbox.push(int(3));

    // Skip first message
    mbox.advance_save();

    let (idx, msg) = mbox.peek_from_save().unwrap();
    assert_eq!(idx, 1);
    assert_eq!(msg, int(2));
}

#[test]
fn remove_at_save_removes_current() {
    let mut mbox = Mailbox::new();
    mbox.push(int(1));
    mbox.push(int(2));
    mbox.push(int(3));

    // Skip first, remove second
    mbox.advance_save();
    let removed = mbox.remove_at_save().unwrap();
    assert_eq!(removed, int(2));
    assert_eq!(mbox.len(), 2);

    // After removal, save_start resets to 0 (BEAM semantics: next receive scans from head)
    assert_eq!(mbox.save_position(), 0);
    let (idx, msg) = mbox.peek_from_save().unwrap();
    assert_eq!(idx, 0);
    assert_eq!(msg, int(1));
}

#[test]
fn selective_receive_skip_and_accept() {
    let mut mbox = Mailbox::new();
    mbox.push(int(10)); // not matching
    mbox.push(int(20)); // not matching
    mbox.push(int(30)); // this one matches

    // Simulate selective receive: skip 10, skip 20, accept 30
    mbox.reset_save();

    // Check 10 — no match, skip
    let (_, msg) = mbox.peek_from_save().unwrap();
    assert_eq!(msg, int(10));
    mbox.advance_save();

    // Check 20 — no match, skip
    let (_, msg) = mbox.peek_from_save().unwrap();
    assert_eq!(msg, int(20));
    mbox.advance_save();

    // Check 30 — match, remove
    let (_, msg) = mbox.peek_from_save().unwrap();
    assert_eq!(msg, int(30));
    let accepted = mbox.remove_at_save().unwrap();
    assert_eq!(accepted, int(30));

    // Remaining: [10, 20]
    assert_eq!(mbox.len(), 2);
}

#[test]
fn reset_save_resets_to_beginning() {
    let mut mbox = Mailbox::new();
    mbox.push(int(1));
    mbox.push(int(2));

    mbox.advance_save();
    mbox.advance_save();
    assert!(mbox.peek_from_save().is_none()); // exhausted

    mbox.reset_save();
    let (idx, msg) = mbox.peek_from_save().unwrap();
    assert_eq!(idx, 0);
    assert_eq!(msg, int(1));
}

#[test]
fn peek_past_end_returns_none() {
    let mut mbox = Mailbox::new();
    mbox.push(int(1));

    mbox.advance_save();
    assert!(mbox.peek_from_save().is_none());

    // Advancing past end is safe (no-op)
    mbox.advance_save();
    assert!(mbox.peek_from_save().is_none());
}

#[test]
fn remove_at_save_past_end_returns_none() {
    let mut mbox = Mailbox::new();
    mbox.push(int(1));

    mbox.advance_save();
    assert!(mbox.remove_at_save().is_none());
}

#[test]
fn message_ordering_preserved() {
    let mut mbox = Mailbox::new();
    for i in 0..10 {
        mbox.push(int(i));
    }

    mbox.reset_save();
    for i in 0..10 {
        let (_, msg) = mbox.peek_from_save().unwrap();
        assert_eq!(msg, int(i));
        mbox.advance_save();
    }
}

#[test]
fn messages_accessor_for_gc() {
    let mut mbox = Mailbox::new();
    mbox.push(int(1));
    mbox.push(int(2));

    let msgs = mbox.messages();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0], int(1));
    assert_eq!(msgs[1], int(2));
}

#[test]
fn save_position_accessor() {
    let mut mbox = Mailbox::new();
    mbox.push(int(1));
    mbox.push(int(2));

    assert_eq!(mbox.save_position(), 0);
    mbox.advance_save();
    assert_eq!(mbox.save_position(), 1);
    mbox.reset_save();
    assert_eq!(mbox.save_position(), 0);
}

#[test]
fn push_during_selective_receive() {
    let mut mbox = Mailbox::new();
    mbox.push(int(1));
    mbox.push(int(2));

    // Start scanning
    mbox.advance_save(); // past msg1

    // New message arrives during receive
    mbox.push(int(3));

    // Continue scanning — should see msg2 next, then msg3
    let (_, msg) = mbox.peek_from_save().unwrap();
    assert_eq!(msg, int(2));
    mbox.advance_save();

    let (_, msg) = mbox.peek_from_save().unwrap();
    assert_eq!(msg, int(3));
}
