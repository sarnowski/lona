// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Iterator implementation for the HAMT.

use alloc::vec::Vec;

use super::node::{Child, Node};

/// State for the HAMT iterator.
pub(super) enum IterState<'hamt, K, V> {
    /// A node to traverse.
    Node(&'hamt Node<K, V>),
    /// A child to process.
    Child(&'hamt Child<K, V>),
    /// Entries in a collision bucket.
    Collision {
        entries: &'hamt [(K, V)],
        index: usize,
    },
}

/// Iterator over HAMT entries.
pub struct HamtIter<'hamt, K, V> {
    stack: Vec<IterState<'hamt, K, V>>,
}

impl<'hamt, K, V> HamtIter<'hamt, K, V> {
    /// Creates a new iterator with the given initial stack.
    pub(super) const fn new(stack: Vec<IterState<'hamt, K, V>>) -> Self {
        Self { stack }
    }
}

impl<'hamt, K, V> Iterator for HamtIter<'hamt, K, V> {
    type Item = (&'hamt K, &'hamt V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let state = self.stack.pop()?;

            match state {
                IterState::Node(node) => match *node {
                    Node::Branch { ref children, .. } => {
                        // Push children in reverse order so we iterate forward
                        for child in children.iter().rev() {
                            self.stack.push(IterState::Child(child));
                        }
                    }
                    Node::Collision { ref entries, .. } => {
                        self.stack.push(IterState::Collision { entries, index: 0 });
                    }
                },
                IterState::Child(child) => match *child {
                    Child::Leaf {
                        ref key, ref value, ..
                    } => {
                        return Some((key, value));
                    }
                    Child::Node(ref node) => {
                        self.stack.push(IterState::Node(node));
                    }
                },
                IterState::Collision { entries, index } => {
                    if let Some(&(ref key, ref value)) = entries.get(index) {
                        self.stack.push(IterState::Collision {
                            entries,
                            index: index.saturating_add(1),
                        });
                        return Some((key, value));
                    }
                }
            }
        }
    }
}
