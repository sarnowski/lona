// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Lona Root Task
//!
//! This is the initial task that seL4 loads at boot.
//! It initializes the Lona VM runtime and starts the REPL.
//!
//! When compiled with the `e2e-test` feature, this instead runs
//! the E2E test suite and reports results via serial output.

#![no_std]
#![no_main]

use core::result::Result::Err;

use core::cell::UnsafeCell;

use lona_vm::UartExt;
use lona_vm::platform::Sel4VSpace;
use lona_vm::{Heap, Vaddr};
use sel4_root_task::root_task;

#[cfg(target_arch = "x86_64")]
use lona_vm::uart::Com1Uart;
#[cfg(target_arch = "aarch64")]
use lona_vm::uart::Pl011Uart;

/// Heap buffer size (64KB).
const HEAP_BUFFER_SIZE: usize = 64 * 1024;

/// Static heap buffer.
/// This is used as the backing memory for the REPL's heap allocator.
/// We use UnsafeCell to safely obtain a mutable pointer without triggering
/// the static_mut_refs lint.
static HEAP_BUFFER: HeapBuffer = HeapBuffer::new();

/// Wrapper around a byte buffer to safely get a mutable pointer.
struct HeapBuffer(UnsafeCell<[u8; HEAP_BUFFER_SIZE]>);

impl HeapBuffer {
    const fn new() -> Self {
        Self(UnsafeCell::new([0; HEAP_BUFFER_SIZE]))
    }

    /// Get a mutable pointer to the buffer.
    ///
    /// # Safety
    ///
    /// Caller must ensure exclusive access.
    const fn as_mut_ptr(&self) -> *mut u8 {
        self.0.get().cast()
    }
}

// SAFETY: We only access HEAP_BUFFER from the single root task thread.
unsafe impl Sync for HeapBuffer {}

/// Entry point for the Lona root task.
///
/// This function is called by seL4 after the kernel has initialized.
/// It receives boot information containing memory regions, capabilities,
/// and other system configuration.
#[root_task]
fn main(bootinfo: &sel4::BootInfoPtr) -> ! {
    // Initialize the Lona VM runtime
    if let Err(_e) = lona_vm::init() {
        sel4::init_thread::suspend_self()
    }

    // Get boot info (currently unused, but available for future use)
    let _info = bootinfo.inner();

    // Initialize platform-specific UART
    #[cfg(target_arch = "x86_64")]
    let mut uart = {
        use lona_vm::uart::{COM1_PORT, x86_64_init};

        // COM1 port range: 0x3F8-0x3FF (8 registers)
        const COM1_FIRST_PORT: u64 = COM1_PORT as u64;
        const COM1_LAST_PORT: u64 = COM1_FIRST_PORT + 7;

        // Get IOPortControl from root task's initial capabilities
        let io_port_control = sel4::init_thread::slot::IO_PORT_CONTROL.cap();

        // Get a slot for the IOPort capability from empty slots
        let empty_slots = bootinfo.empty();
        let slot_range = empty_slots.range();
        let ioport_slot_index = slot_range.start;

        // Create destination for the IOPort capability
        let root_cnode = sel4::init_thread::slot::CNODE.cap();
        let ioport_slot_cptr = sel4::CPtr::from_bits(ioport_slot_index as u64);
        let dst = root_cnode.absolute_cptr(ioport_slot_cptr);

        // Issue IOPort capability for COM1 ports
        match io_port_control.ioport_control_issue(COM1_FIRST_PORT, COM1_LAST_PORT, &dst) {
            Ok(()) => {
                // Initialize UART with the capability
                // SAFETY: Single-threaded context, valid capability
                unsafe { x86_64_init(ioport_slot_cptr.bits()) };
            }
            Err(e) => {
                sel4::debug_println!("Failed to issue IOPort capability: {:?}", e);
            }
        }
        Com1Uart::new()
    };

    #[cfg(target_arch = "aarch64")]
    let mut uart = {
        // Map UART device memory before we can use it
        use lona_vm::platform::mmio;
        use lona_vm::uart::{PL011_PADDR, aarch64_init};

        // SAFETY: bootinfo is valid, single-threaded context
        if let Some(vaddr) = unsafe { mmio::map_device_frame(bootinfo, PL011_PADDR) } {
            // SAFETY: vaddr points to valid mapped UART memory
            unsafe { aarch64_init(vaddr.as_mut_ptr::<u8>()) };
        } else {
            sel4::debug_println!("Failed to map UART - using debug output only");
        }
        Pl011Uart::new()
    };

    // Early boot logging
    uart.write_str("Lona ");
    uart.write_str(lona_vm::VERSION);
    uart.write_str("\n");

    // Set up heap using static buffer
    // SAFETY: We have exclusive access to HEAP_BUFFER in the root task
    let heap_base = {
        let ptr = HEAP_BUFFER.as_mut_ptr();
        // Heap base is at top of buffer (heap grows down)
        Vaddr::new(ptr as u64 + HEAP_BUFFER_SIZE as u64)
    };
    let heap_size = HEAP_BUFFER_SIZE;
    let mut heap = Heap::new(heap_base, heap_size);

    // Create VSpace (Sel4VSpace interprets addresses directly)
    let mut mem = Sel4VSpace;

    // Log initialization complete
    uart.write_str("Lona initialized.\n");

    // List embedded library contents
    uart.write_str("\nEmbedded libraries:\n");
    match lona_vm::TarSource::embedded() {
        Ok(source) => {
            for entry in source.entries() {
                // Get filename - need to bind TarFormatString to extend its lifetime
                let filename_tar = entry.filename();
                let Ok(filename) = filename_tar.as_str() else {
                    continue;
                };
                // Skip directories
                if filename.ends_with('/') {
                    continue;
                }
                uart.write_str("  ");
                uart.write_str(filename);
                uart.write_str(" (");
                print_size(&mut uart, entry.data().len());
                uart.write_str(" bytes)\n");
            }

            // TODO: Load bootstrap namespace once reader/evaluator are ready
            // use lona_vm::NamespaceSource;
            // let core_bytes = source.resolve("lona.core")
            //     .expect("lona.core not found in embedded archive");
            // let core_str = core::str::from_utf8(core_bytes)
            //     .expect("lona.core is not valid UTF-8");
            // // Read and evaluate all forms from lona.core
            // // let forms = lona_vm::reader::read_all(core_str);
            // // for form in forms { evaluator.eval(form); }
        }
        Err(_) => {
            uart.write_str("  ERROR: Failed to load embedded archive\n");
        }
    }
    uart.write_str("\n");

    // Branch: E2E tests or normal REPL
    #[cfg(feature = "e2e-test")]
    {
        run_e2e_tests(&mut heap, &mut mem, &mut uart);
    }

    #[cfg(not(feature = "e2e-test"))]
    {
        run_repl(&mut heap, &mut mem, &mut uart);
    }
}

/// Normal boot sequence: run the REPL.
#[cfg(not(feature = "e2e-test"))]
fn run_repl<U: lona_vm::Uart>(heap: &mut Heap, mem: &mut Sel4VSpace, uart: &mut U) -> ! {
    // Run the REPL (never returns)
    lona_vm::repl::run(heap, mem, uart)
}

/// Run E2E tests and report results.
#[cfg(feature = "e2e-test")]
fn run_e2e_tests<U: lona_vm::Uart>(heap: &mut Heap, mem: &mut Sel4VSpace, uart: &mut U) -> ! {
    sel4::debug_println!("");
    sel4::debug_println!("========================================");
    sel4::debug_println!("  Lona E2E Test Suite");
    sel4::debug_println!("  Version: {}", lona_vm::VERSION);
    sel4::debug_println!("========================================");
    sel4::debug_println!("");

    // Run all E2E tests using the same heap/mem/uart as the REPL would use
    let all_passed = lona_vm::e2e::run_all_tests(heap, mem, uart);

    sel4::debug_println!("");
    if all_passed {
        sel4::debug_println!("All tests passed!");
    } else {
        sel4::debug_println!("Some tests failed!");
    }
    sel4::debug_println!("");

    // Suspend - test harness will detect completion and kill QEMU
    sel4::init_thread::suspend_self()
}

/// Print a decimal number to UART.
///
/// Since we don't have `format!` in no_std, this manually converts
/// the number to decimal digits.
fn print_size<U: lona_vm::Uart>(uart: &mut U, mut n: usize) {
    if n == 0 {
        uart.write_str("0");
        return;
    }
    // Maximum digits for usize on 64-bit
    const MAX_DIGITS: usize = 20;
    let mut digits = [0u8; MAX_DIGITS];
    let mut i = 0;
    while n > 0 {
        digits[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    // Print digits in reverse order
    while i > 0 {
        i -= 1;
        uart.write_byte(digits[i]);
    }
}
