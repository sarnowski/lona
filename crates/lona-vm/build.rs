// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Build script for Lona VM.
//!
//! - Creates lonalib.tar from lib/ directory (embedded at compile time)
//! - Adds linker script to place VM at `SHARED_CODE_BASE`

fn main() {
    let Ok(out_dir) = std::env::var("OUT_DIR") else {
        eprintln!("error: OUT_DIR not set");
        std::process::exit(1);
    };
    let tar_path = format!("{out_dir}/lonalib.tar");

    // lib/ directory is at workspace root (../../lib from this crate)
    let lib_dir = "../../lib";

    // Create tar archive from lib/ directory
    // Use --format=ustar for compatibility with tar-no-std crate
    let Ok(status) = std::process::Command::new("tar")
        .args(["--format=ustar", "-cf", &tar_path, "-C", lib_dir, "."])
        .status()
    else {
        eprintln!("error: failed to run tar command");
        std::process::exit(1);
    };

    if !status.success() {
        eprintln!("error: tar command failed");
        std::process::exit(1);
    }

    // Rerun if lib/ contents change
    println!("cargo::rerun-if-changed={lib_dir}/");

    // Add linker script for seL4 builds (places VM at SHARED_CODE_BASE)
    // Only apply to seL4 targets, not test builds (which use std)
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.ends_with("-sel4") {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let linker_script = format!("{manifest_dir}/lona-vm.ld");
        if std::path::Path::new(&linker_script).exists() {
            println!("cargo::rustc-link-arg=-T{linker_script}");
            println!("cargo::rerun-if-changed={linker_script}");
        }
    }
}
