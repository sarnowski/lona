// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Flattened Device Tree (FDT) parsing for device discovery.
//!
//! seL4 provides the FDT in bootinfo extras, describing all hardware devices.
//! This module parses the FDT to discover device addresses dynamically,
//! enabling multi-platform support without hardcoded addresses.

use sel4::{BootInfoExtraId, BootInfoPtr};

/// Information about a UART device discovered from the FDT.
#[derive(Debug, Clone, Copy)]
pub struct UartInfo {
    /// Physical base address of the UART registers.
    pub paddr: usize,
    /// Size of the UART register region in bytes.
    pub size: usize,
}

/// Error type for FDT parsing operations.
#[derive(Debug, Clone, Copy)]
pub enum DiscoveryError {
    /// FDT not found in bootinfo extras.
    NotFound,
    /// FDT data is invalid or cannot be parsed.
    InvalidFdt,
    /// UART device not found in FDT.
    UartNotFound,
}

/// Discovers UART information from the bootinfo FDT.
///
/// Searches the device tree (passed via bootinfo extras) for a UART device by:
/// 1. Following the chosen/stdout-path (preferred method)
/// 2. Searching for nodes with compatible = "arm,pl011"
///
/// Returns the UART's physical address and size.
pub fn discover_uart(bootinfo: &BootInfoPtr) -> Result<UartInfo, DiscoveryError> {
    for extra in bootinfo.extra() {
        if extra.id == BootInfoExtraId::Fdt {
            let fdt_bytes = extra.content();
            if fdt_bytes.is_empty() {
                continue;
            }

            let fdt = fdt::Fdt::new(fdt_bytes).map_err(|_err| DiscoveryError::InvalidFdt)?;

            // Strategy 1: Try chosen/stdout-path first (most reliable)
            if let Some(uart_info) = find_uart_from_chosen(&fdt) {
                return Ok(uart_info);
            }

            // Strategy 2: Search for PL011 UART by compatible string
            if let Some(uart_info) = find_uart_by_compatible(&fdt) {
                return Ok(uart_info);
            }

            return Err(DiscoveryError::UartNotFound);
        }
    }

    Err(DiscoveryError::NotFound)
}

/// Attempts to find UART via chosen/stdout-path.
fn find_uart_from_chosen(fdt: &fdt::Fdt) -> Option<UartInfo> {
    let chosen = fdt.chosen();
    let stdout = chosen.stdout()?;
    let mut reg = stdout.reg()?;
    let region = reg.next()?;

    #[expect(
        clippy::as_conversions,
        reason = "FDT starting_address is a pointer that must be converted to physical address"
    )]
    let paddr = region.starting_address as usize;

    Some(UartInfo {
        paddr,
        size: region.size.unwrap_or(0x1000),
    })
}

/// Attempts to find PL011 UART by compatible string.
fn find_uart_by_compatible(fdt: &fdt::Fdt) -> Option<UartInfo> {
    for node in fdt.all_nodes() {
        let Some(compatible) = node.compatible() else {
            continue;
        };

        for compat in compatible.all() {
            if compat == "arm,pl011" || compat == "arm,primecell" {
                let mut reg = node.reg()?;
                let region = reg.next()?;

                #[expect(
                    clippy::as_conversions,
                    reason = "FDT starting_address is a pointer that must be converted to physical address"
                )]
                let paddr = region.starting_address as usize;

                return Some(UartInfo {
                    paddr,
                    size: region.size.unwrap_or(0x1000),
                });
            }
        }
    }

    None
}
