// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Build script to create lonalib.tar from lib/ directory.
//!
//! The tar archive is embedded into the root task ELF at compile time.

// Build scripts need different lint settings than the main crate.
// Panicking on error is appropriate here since build failures should halt compilation.
#![allow(clippy::expect_used, clippy::panic)]

fn main() {
    let Ok(out_dir) = std::env::var("OUT_DIR") else {
        panic!("OUT_DIR not set");
    };
    let tar_path = format!("{out_dir}/lonalib.tar");

    // Create tar archive from lib/ directory
    // Use --format=ustar for compatibility with tar-no-std crate
    let Ok(status) = std::process::Command::new("tar")
        .args(["--format=ustar", "-cf", &tar_path, "-C", "lib", "."])
        .status()
    else {
        panic!("failed to run tar command");
    };

    assert!(status.success(), "tar command failed");

    // Rerun if lib/ contents change
    println!("cargo::rerun-if-changed=lib/");
}
