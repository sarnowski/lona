// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Insert operations for the HAMT.
//!
//! Contains all the recursive operations for inserting key-value pairs
//! into the trie, including handling collisions and merging leaves.

use alloc::rc::Rc;
use alloc::vec::Vec;

use core::hash::Hash;

use super::node::{Child, MAX_DEPTH, Node, bitmap_to_index, index_at_depth};

/// Inserts into a collision node.
///
/// When keys have the same hash, they're stored together in a collision
/// node. This function either updates an existing key or adds a new one.
///
/// Returns `(new_node, was_added)` where `was_added` is `true` if a new
/// entry was added rather than updating an existing one.
pub(super) fn insert_into_collision<K, V>(
    collision_hash: u64,
    entries: &[(K, V)],
    key: K,
    value: V,
) -> (Node<K, V>, bool)
where
    K: Clone + Eq,
    V: Clone,
{
    let mut new_entries: Vec<(K, V)> = entries.to_vec();
    let mut found = false;

    for &mut (ref entry_key, ref mut entry_value) in &mut new_entries {
        if *entry_key == key {
            *entry_value = value.clone();
            found = true;
            break;
        }
    }

    if !found {
        new_entries.push((key, value));
    }

    let new_node = Node::Collision {
        hash: collision_hash,
        entries: Rc::from(new_entries),
    };
    (new_node, !found)
}

/// Recursively inserts a key-value pair into the trie.
///
/// Navigates through the trie using hash bits at each depth, creating
/// new nodes along the path with path copying for immutability.
///
/// Returns `(new_node, was_added)` where `was_added` is `true` if a new
/// entry was added rather than updating an existing one.
pub(super) fn insert_rec<K, V>(
    node: &Node<K, V>,
    key: K,
    value: V,
    hash: u64,
    depth: u32,
) -> (Node<K, V>, bool)
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    match *node {
        Node::Branch {
            bitmap,
            ref children,
        } => insert_into_branch(bitmap, children, key, value, hash, depth),
        Node::Collision {
            hash: collision_hash,
            ref entries,
        } => insert_into_collision(collision_hash, entries, key, value),
    }
}

/// Inserts into a branch node.
///
/// Handles the three cases for branch insertion:
/// 1. Empty slot: creates a new leaf
/// 2. Existing leaf: may replace, create collision, or expand to sub-trie
/// 3. Existing sub-node: recursively inserts
///
/// Returns `(new_node, was_added)` where `was_added` is `true` if a new
/// entry was added rather than updating an existing one.
fn insert_into_branch<K, V>(
    bitmap: u32,
    children: &Rc<[Child<K, V>]>,
    key: K,
    value: V,
    hash: u64,
    depth: u32,
) -> (Node<K, V>, bool)
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    let idx = index_at_depth(hash, depth);
    let bit = 1_u32 << idx;
    let child_idx = bitmap_to_index(bitmap, idx);

    if bitmap & bit == 0 {
        // No entry at this index, add new leaf
        let new_leaf = Child::Leaf { hash, key, value };
        let mut new_children: Vec<Child<K, V>> = children.iter().cloned().collect();
        new_children.insert(child_idx, new_leaf);
        let new_node = Node::Branch {
            bitmap: bitmap | bit,
            children: Rc::from(new_children),
        };
        return (new_node, true);
    }

    // Entry exists at this index
    let mut new_children: Vec<Child<K, V>> = children.iter().cloned().collect();
    let Some(existing_child) = new_children.get(child_idx) else {
        return (
            Node::Branch {
                bitmap,
                children: Rc::clone(children),
            },
            false,
        );
    };

    let (new_child, added) = match *existing_child {
        Child::Leaf {
            hash: leaf_hash,
            key: ref leaf_key,
            value: ref leaf_value,
        } => insert_at_leaf(leaf_hash, leaf_key, leaf_value, key, value, hash, depth),
        Child::Node(ref sub_node) => {
            let (new_sub, added) = insert_rec(sub_node, key, value, hash, depth.saturating_add(1));
            (Child::Node(Rc::new(new_sub)), added)
        }
    };

    if let Some(slot) = new_children.get_mut(child_idx) {
        *slot = new_child;
    }
    let new_node = Node::Branch {
        bitmap,
        children: Rc::from(new_children),
    };
    (new_node, added)
}

/// Handles insertion at a leaf node.
///
/// When inserting at an existing leaf, there are three possibilities:
/// - Same key: replace the value (no size change)
/// - Same hash, different key: create a collision node
/// - Different hash: expand into a sub-trie via [`merge_leaves`]
///
/// Returns `(new_child, was_added)` where `was_added` is `true` if a new
/// entry was added rather than updating an existing one.
fn insert_at_leaf<K, V>(
    leaf_hash: u64,
    leaf_key: &K,
    leaf_value: &V,
    key: K,
    value: V,
    hash: u64,
    depth: u32,
) -> (Child<K, V>, bool)
where
    K: Clone + Eq,
    V: Clone,
{
    if *leaf_key == key {
        // Replace existing value
        (Child::Leaf { hash, key, value }, false)
    } else if leaf_hash == hash {
        // Hash collision - create collision node
        let collision = Node::Collision {
            hash,
            entries: Rc::from([(leaf_key.clone(), leaf_value.clone()), (key, value)]),
        };
        (Child::Node(Rc::new(collision)), true)
    } else {
        // Different hashes - expand to sub-trie
        let existing_leaf = Child::Leaf {
            hash: leaf_hash,
            key: leaf_key.clone(),
            value: leaf_value.clone(),
        };
        let new_leaf = Child::Leaf { hash, key, value };
        let sub_node = merge_leaves(existing_leaf, new_leaf, depth.saturating_add(1));
        (Child::Node(Rc::new(sub_node)), true)
    }
}

/// Merges two leaves into a sub-trie at the given depth.
///
/// When two leaves have different hash values but collide at the same
/// position, they are recursively expanded into a sub-trie until their
/// hash bits diverge. If they share all hash bits (extremely unlikely
/// with a good hash function), they become a collision node.
///
/// # Panics
///
/// Debug builds will panic if called with non-leaf children, as this
/// indicates a bug in the HAMT implementation.
fn merge_leaves<K, V>(leaf1: Child<K, V>, leaf2: Child<K, V>, depth: u32) -> Node<K, V>
where
    K: Clone + Eq,
    V: Clone,
{
    // Extract leaf data, with defensive handling for non-leaf children
    let (hash1, key1, value1) = match leaf1 {
        Child::Leaf { hash, key, value } => (hash, key, value),
        Child::Node(_) => {
            debug_assert!(false, "merge_leaves called with non-leaf child");
            // In release builds, return an empty branch as a safe fallback
            return Node::Branch {
                bitmap: 0,
                children: Rc::from([]),
            };
        }
    };

    let (hash2, key2, value2) = match leaf2 {
        Child::Leaf { hash, key, value } => (hash, key, value),
        Child::Node(_) => {
            debug_assert!(false, "merge_leaves called with non-leaf child");
            return Node::Branch {
                bitmap: 0,
                children: Rc::from([]),
            };
        }
    };

    // If we've exhausted all hash bits, create a collision node
    // (depth >= MAX_DEPTH because at depth=13, shift would be 65 bits)
    if depth >= MAX_DEPTH {
        return Node::Collision {
            hash: hash1,
            entries: Rc::from([(key1, value1), (key2, value2)]),
        };
    }

    let idx1 = index_at_depth(hash1, depth);
    let idx2 = index_at_depth(hash2, depth);

    // Reconstruct leaves from extracted values
    let new_leaf1 = Child::Leaf {
        hash: hash1,
        key: key1,
        value: value1,
    };
    let new_leaf2 = Child::Leaf {
        hash: hash2,
        key: key2,
        value: value2,
    };

    if idx1 == idx2 {
        // Same index at this level - recurse deeper
        let sub_node = merge_leaves(new_leaf1, new_leaf2, depth.saturating_add(1));
        let bit = 1_u32 << idx1;
        Node::Branch {
            bitmap: bit,
            children: Rc::from([Child::Node(Rc::new(sub_node))]),
        }
    } else {
        // Different indices - create branch with both leaves
        let (bit1, bit2) = (1_u32 << idx1, 1_u32 << idx2);
        let bitmap = bit1 | bit2;
        let children = if idx1 < idx2 {
            Rc::from([new_leaf1, new_leaf2])
        } else {
            Rc::from([new_leaf2, new_leaf1])
        };
        Node::Branch { bitmap, children }
    }
}
