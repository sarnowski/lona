// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Scheduler for BEAM-style lightweight processes.
//!
//! This module implements per-worker run queues and a process table for
//! multiplexing Lonala processes within a realm.

mod core;
mod exit_propagation;
pub(crate) mod process_table;
mod run_queue;
mod worker;

#[cfg(test)]
mod process_table_test;
#[cfg(test)]
mod run_queue_test;
#[cfg(test)]
mod scheduler_test;
#[cfg(test)]
mod worker_test;

pub use self::core::{DEFAULT_WORKER_COUNT, Scheduler, TickResult};
pub use process_table::{MAX_SEGMENTS, ProcessTable, SEGMENT_SIZE};
pub use run_queue::{RUN_QUEUE_CAPACITY, RunQueue};
pub use worker::Worker;
