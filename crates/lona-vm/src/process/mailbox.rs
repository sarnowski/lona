// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Per-process mailbox for message passing.
//!
//! Each process has a mailbox that stores incoming messages as `Term` values.
//! The mailbox supports selective receive: a scan position (`save_start`)
//! tracks which messages have been examined. Non-matching messages are skipped,
//! and the matching message is removed without disturbing others.
//!
//! ```text
//! Mailbox: [msg0] [msg1] [msg2] [msg3]
//!                   ^save_start
//!
//! peek_from_save() → (1, msg1)
//! advance_save()   → save_start moves to 2
//! remove_at_save() → removes msg2, save_start resets to 0
//! reset_save()     → save_start back to 0
//! ```

extern crate alloc;

use alloc::collections::VecDeque;

use crate::term::Term;

/// Per-process mailbox with selective receive support.
///
/// Messages are stored in insertion order. The `save_start` position
/// supports selective receive by tracking which messages have already
/// been examined in the current receive operation.
pub struct Mailbox {
    /// Message queue in arrival order.
    messages: VecDeque<Term>,
    /// Scan position for selective receive (index into `messages`).
    save_start: usize,
}

impl Mailbox {
    /// Create a new empty mailbox.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            messages: VecDeque::new(),
            save_start: 0,
        }
    }

    /// Append a message to the end of the mailbox.
    pub fn push(&mut self, msg: Term) {
        self.messages.push_back(msg);
    }

    /// Number of messages in the mailbox.
    #[must_use]
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if the mailbox is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Peek at the message at the current save position.
    ///
    /// Returns `Some((index, term))` if there is a message at `save_start`,
    /// or `None` if all messages have been examined.
    #[must_use]
    pub fn peek_from_save(&self) -> Option<(usize, Term)> {
        if self.save_start < self.messages.len() {
            Some((self.save_start, self.messages[self.save_start]))
        } else {
            None
        }
    }

    /// Advance the save position past a non-matching message.
    pub fn advance_save(&mut self) {
        if self.save_start < self.messages.len() {
            self.save_start += 1;
        }
    }

    /// Remove the message at the current save position (match accepted).
    ///
    /// Resets `save_start` to 0 after removal, matching BEAM's
    /// `remove_message` which always resets the save pointer to the
    /// head of the queue. The next `receive` scan starts from the beginning.
    pub fn remove_at_save(&mut self) -> Option<Term> {
        if self.save_start < self.messages.len() {
            let msg = self.messages.remove(self.save_start);
            self.save_start = 0;
            msg
        } else {
            None
        }
    }

    /// Reset the scan position to the beginning.
    ///
    /// Called at the start of each new `receive` operation.
    pub const fn reset_save(&mut self) {
        self.save_start = 0;
    }

    /// Get the current save position.
    #[must_use]
    pub const fn save_position(&self) -> usize {
        self.save_start
    }

    /// Access the message queue for GC root scanning.
    #[must_use]
    pub const fn messages(&self) -> &VecDeque<Term> {
        &self.messages
    }

    /// Mutable access to the message queue for GC updates.
    pub const fn messages_mut(&mut self) -> &mut VecDeque<Term> {
        &mut self.messages
    }
}

impl Default for Mailbox {
    fn default() -> Self {
        Self::new()
    }
}
