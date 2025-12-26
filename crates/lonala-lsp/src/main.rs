// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Lonala Language Server entry point.

use std::env;
use std::io::Write as _;
use tower_lsp::{LspService, Server};
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;

mod document;
mod semantic_tokens;
mod server;

use server::Lonala;

fn print_help() {
    let mut stderr = std::io::stderr();
    drop(writeln!(stderr, "lonala-lsp - Language Server for Lonala"));
    drop(writeln!(stderr));
    drop(writeln!(stderr, "USAGE:"));
    drop(writeln!(stderr, "    lonala-lsp [OPTIONS]"));
    drop(writeln!(stderr));
    drop(writeln!(stderr, "OPTIONS:"));
    drop(writeln!(
        stderr,
        "    -h, --help       Print this help message"
    ));
    drop(writeln!(
        stderr,
        "    -V, --version    Print version information"
    ));
    drop(writeln!(stderr));
    drop(writeln!(
        stderr,
        "The server communicates via stdin/stdout using the LSP protocol."
    ));
}

fn print_version() {
    let mut stderr = std::io::stderr();
    drop(writeln!(stderr, "lonala-lsp {}", env!("CARGO_PKG_VERSION")));
}

#[tokio::main]
async fn main() {
    // Handle --help and --version before starting server
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_help();
        return;
    }
    if args.iter().any(|arg| arg == "-V" || arg == "--version") {
        print_version();
        return;
    }

    // Initialize tracing - writes to stderr (stdout is for LSP JSON-RPC)
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Lonala::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
