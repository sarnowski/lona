// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Build script for lona-runtime.
//!
//! Propagates the `LONA_VERSION` environment variable to the Rust code
//! so it can be accessed at compile time via `env!("LONA_VERSION")`.

fn main() {
    // Read version from environment, default to "unknown" if not set
    let version = std::env::var("LONA_VERSION").unwrap_or_else(|_| String::from("unknown"));

    // Make version available to Rust code via env!("LONA_VERSION")
    println!("cargo::rustc-env=LONA_VERSION={version}");

    // Rerun if version changes
    println!("cargo::rerun-if-env-changed=LONA_VERSION");
}
