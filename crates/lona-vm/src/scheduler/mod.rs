// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Scheduler for BEAM-style lightweight processes.
//!
//! This module implements per-worker run queues and a process table for
//! multiplexing Lonala processes within a realm.

mod core;
mod process_table;
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

pub use self::core::{Scheduler, TickResult};
pub use process_table::{MAX_PROCESSES, ProcessTable};
pub use run_queue::{RUN_QUEUE_CAPACITY, RunQueue};
pub use worker::Worker;
