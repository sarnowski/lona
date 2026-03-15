// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Lona VM Entry Point
//!
//! This is the entry point for all Lona realms. The Lona Memory Manager
//! starts a TCB with PC pointing here after creating a realm.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use core::arch::asm;
use core::panic::PanicInfo;

// Global allocator for heap allocations (Vec, etc.)
// Uses a simple bump allocator with a fixed-size buffer.
mod allocator {
    use core::alloc::{GlobalAlloc, Layout};
    use core::cell::UnsafeCell;
    use core::ptr;
    use core::sync::atomic::{AtomicUsize, Ordering};

    /// Size of the allocation buffer (128 KB for Realm struct + bytecode chunks).
    const ALLOC_SIZE: usize = 128 * 1024;

    /// Simple bump allocator for no_std environments.
    pub struct BumpAllocator {
        buffer: UnsafeCell<[u8; ALLOC_SIZE]>,
        next: AtomicUsize,
    }

    // SAFETY: alloc() uses atomic compare_exchange_weak on the bump pointer,
    // ensuring thread-safe allocation across concurrent worker TCBs.
    // dealloc() is a no-op (bump allocator never frees).
    unsafe impl Sync for BumpAllocator {}

    impl BumpAllocator {
        pub const fn new() -> Self {
            Self {
                buffer: UnsafeCell::new([0u8; ALLOC_SIZE]),
                next: AtomicUsize::new(0),
            }
        }
    }

    unsafe impl GlobalAlloc for BumpAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let size = layout.size();
            let align = layout.align();

            loop {
                let current = self.next.load(Ordering::Relaxed);
                let buffer_start = self.buffer.get() as usize;
                let alloc_start = buffer_start + current;

                // Align up
                let aligned = (alloc_start + align - 1) & !(align - 1);
                let offset_from_buffer = aligned - buffer_start;
                let new_next = offset_from_buffer + size;

                if new_next > ALLOC_SIZE {
                    return ptr::null_mut();
                }

                if self
                    .next
                    .compare_exchange_weak(current, new_next, Ordering::SeqCst, Ordering::Relaxed)
                    .is_ok()
                {
                    return aligned as *mut u8;
                }
            }
        }

        unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
            // Bump allocator doesn't support deallocation
        }
    }

    #[global_allocator]
    static ALLOCATOR: BumpAllocator = BumpAllocator::new();
}

use lona_abi::BootFlags;
#[cfg(target_arch = "aarch64")]
use lona_abi::layout::UART_VADDR;
use lona_vm::Vaddr;
#[cfg(feature = "e2e-test")]
use lona_vm::e2e;
use lona_vm::loader::TarSource;
#[cfg(not(any(test, feature = "std")))]
use lona_vm::platform::Sel4VSpace;
#[cfg(not(feature = "e2e-test"))]
use lona_vm::process::WorkerId;
use lona_vm::process::pool::ProcessPool;
use lona_vm::process::{INITIAL_OLD_HEAP_SIZE, INITIAL_YOUNG_HEAP_SIZE, Process};
use lona_vm::realm::Realm;
#[cfg(not(feature = "e2e-test"))]
use lona_vm::repl;
#[cfg(not(feature = "e2e-test"))]
use lona_vm::scheduler::Worker;
use lona_vm::uart::{Uart, UartExt};

#[cfg(target_arch = "aarch64")]
use lona_vm::uart::Pl011Uart;

#[cfg(all(target_arch = "x86_64", feature = "sel4"))]
use lona_abi::layout::worker_ipc_buffer;
#[cfg(all(target_arch = "x86_64", feature = "sel4"))]
use lona_abi::types::CapSlot;
#[cfg(all(target_arch = "x86_64", feature = "sel4"))]
use lona_vm::uart::Com1Uart;

/// Entry point called when TCB is resumed.
///
/// Boot arguments are passed as function parameters. On x86_64, these come from:
/// - `realm_id`: RDI
/// - `worker_id`: RSI
/// - `heap_start`: RDX
/// - `heap_size`: RCX
/// - `flags`: R8
///
/// On aarch64, these come from X0-X4.
///
/// The LMM writes these values to the TCB's registers before resuming it.
/// By declaring them as function parameters, we force LLVM to treat them as
/// real ABI inputs and preserve them across the function prologue.
#[unsafe(no_mangle)]
pub extern "C" fn _start(
    realm_id: u64,
    worker_id: u64,
    heap_start: u64,
    heap_size: u64,
    flags: u64,
) -> ! {
    // Worker ID is passed as u64 but is guaranteed to fit in u16 (max MAX_WORKERS per realm)
    let worker_id = (worker_id & 0xFFFF) as u16;

    let boot_flags = BootFlags::new(flags);

    // Initialize UART based on platform (also sets up IPC buffer)
    #[cfg(target_arch = "aarch64")]
    let mut uart = init_uart_aarch64(boot_flags, worker_id);

    #[cfg(all(target_arch = "x86_64", feature = "sel4"))]
    let mut uart = init_uart_x86_64(boot_flags, worker_id);

    // Non-bootstrap workers: set up platform, then idle.
    // Only Worker 0 bootstraps the realm and runs REPL/E2E.
    // Workers 1-3 will participate in scheduling once the Scheduler
    // is shared across workers (M2 Phase 2E).
    if worker_id != 0 {
        idle_loop();
    }

    // --- Worker 0 only below this point ---

    // Print boot info if UART is available
    if boot_flags.has_uart() {
        print_boot_info(
            &mut uart, realm_id, worker_id, heap_start, heap_size, boot_flags,
        );
    }

    // Create process pool from boot-allocated heap memory and allocate code region
    const REALM_CODE_SIZE: usize = 32 * 1024;
    let mut pool = ProcessPool::new(Vaddr::new(heap_start), heap_size as usize);
    let code_base = pool
        .allocate(REALM_CODE_SIZE, 8)
        .expect("failed to allocate realm code region");

    // Box::new(Realm::new(...)) benefits from NRVO since Realm::new is infallible.
    // Realm is ~60KB, so heap allocation avoids stack overflow.
    let mut realm = Box::new(Realm::new(pool, code_base, REALM_CODE_SIZE));

    // Allocate memory for REPL process (young heap + old heap)
    // Uses growth-enabled allocation that requests more pages from LMM if needed
    let (young_base, old_base) = realm
        .allocate_process_memory(INITIAL_YOUNG_HEAP_SIZE, INITIAL_OLD_HEAP_SIZE)
        .expect("failed to allocate REPL process memory");

    // Create REPL process with BEAM-style memory layout
    let mut process = Process::new(
        young_base,
        INITIAL_YOUNG_HEAP_SIZE,
        old_base,
        INITIAL_OLD_HEAP_SIZE,
    );

    let mut vspace = Sel4VSpace;

    // Bootstrap realm with lona.core namespace and essential vars
    let bootstrap_result =
        lona_vm::realm::bootstrap(&mut realm, &mut vspace).expect("failed to bootstrap realm");

    // Bootstrap process with *ns* binding
    process.bootstrap(bootstrap_result.ns_var, bootstrap_result.core_ns);

    // Run E2E tests when feature is enabled, otherwise start REPL
    #[cfg(feature = "e2e-test")]
    {
        e2e::run_all_tests(&mut process, &mut realm, &mut vspace, &mut uart);
        halt_loop()
    }

    #[cfg(not(feature = "e2e-test"))]
    {
        if boot_flags.has_uart() {
            uart.write_line("\nStarting REPL...\n");
        }
        let mut worker = Worker::new(WorkerId(0));

        // Give the REPL process a PID so (self) returns a valid PID term.
        // Index 0, generation 0 — the REPL is always the first process.
        let repl_pid = lona_vm::process::ProcessId::new(0, 0);
        process.pid = repl_pid;
        if let Some(pid_term) = process.alloc_term_pid(&mut vspace, 0, 0) {
            process.pid_term = Some(pid_term);
        }

        repl::run(
            &mut worker,
            &mut process,
            &mut vspace,
            &mut realm,
            &mut uart,
        )
    }
}

/// Idle loop for non-bootstrap workers.
///
/// Workers 1-3 voluntarily yield their seL4 time slice. They don't
/// participate in scheduling until the Scheduler is shared across
/// workers (M2 Phase 2E).
///
/// Uses `seL4_Yield` on both architectures — this surrenders the
/// remaining budget to the kernel scheduler without burning CPU.
fn idle_loop() -> ! {
    loop {
        sel4::r#yield();
    }
}

/// Maximum workers per realm (must match LMM's `MAX_REALM_WORKERS`).
const MAX_WORKERS: usize = 4;

/// TLS block size for aarch64 (matches x86_64 for consistency).
#[cfg(target_arch = "aarch64")]
const TLS_BLOCK_SIZE_AARCH64: usize = 4096;

/// Static TLS blocks for aarch64 (one per worker).
///
/// On aarch64, TLS uses variant 1 where the thread pointer (TPIDR_EL0) points
/// to the TCB at the START of the TLS block. TLS variables are accessed via
/// positive offsets from the thread pointer.
///
/// Each worker TCB needs its own TLS block to have independent thread-local
/// storage (e.g., the seL4 IPC buffer pointer).
#[cfg(target_arch = "aarch64")]
#[repr(C, align(16))]
struct TlsBlockAarch64 {
    /// TCB / DTV pointer (required by TLS ABI, but we can leave as zero)
    tcb: [usize; 2],
    /// Space for TLS data
    data: [u8; TLS_BLOCK_SIZE_AARCH64],
}

#[cfg(target_arch = "aarch64")]
static mut TLS_BLOCKS_AARCH64: [TlsBlockAarch64; MAX_WORKERS] = [const {
    TlsBlockAarch64 {
        tcb: [0; 2],
        data: [0u8; TLS_BLOCK_SIZE_AARCH64],
    }
}; MAX_WORKERS];

/// Initialize TLS on aarch64 by setting TPIDR_EL0 to this worker's TLS block.
#[cfg(target_arch = "aarch64")]
unsafe fn init_tls_aarch64(worker_id: u16) {
    use core::arch::asm;
    unsafe {
        let idx = (worker_id as usize).min(MAX_WORKERS - 1);
        let tls_ptr = core::ptr::addr_of_mut!(TLS_BLOCKS_AARCH64[idx]) as usize;
        asm!("msr tpidr_el0, {}", in(reg) tls_ptr, options(nomem, nostack));
    }
}

/// Initialize UART on aarch64 (PL011 MMIO).
#[cfg(target_arch = "aarch64")]
fn init_uart_aarch64(boot_flags: BootFlags, worker_id: u16) -> Pl011Uart {
    // Initialize TLS first - required for seL4 TLS variables
    // SAFETY: Each worker calls this once with its own worker_id,
    // getting its own TLS block. No concurrent access to the same block.
    unsafe {
        init_tls_aarch64(worker_id);
    }

    // Initialize UART (MMIO-based, no seL4 syscalls needed)
    if boot_flags.has_uart() {
        // SAFETY: Memory manager has mapped UART at UART_VADDR
        unsafe {
            lona_vm::uart::aarch64_init(UART_VADDR as *mut u8);
        }
    }

    // Set up IPC buffer for seL4 syscalls (required before ANY seL4 syscall)
    let ipc_buffer_addr = lona_abi::layout::worker_ipc_buffer(worker_id);
    // SAFETY: Memory manager has mapped IPC buffer at this address and
    // it will remain valid for the lifetime of this VM.
    unsafe {
        let ipc_buffer = &mut *(ipc_buffer_addr as *mut sel4::IpcBuffer);
        sel4::set_ipc_buffer(ipc_buffer);
    }

    Pl011Uart::new()
}

/// Maximum TLS block size in bytes (generous for debug builds).
/// The sel4 crate needs ~16 bytes for IPC buffer pointer, but debug builds
/// may have additional TLS variables from debug assertions and other code.
#[cfg(all(target_arch = "x86_64", feature = "sel4"))]
const TLS_BLOCK_SIZE: usize = 4096;

/// Static TLS blocks for x86_64 (one per worker).
///
/// On x86_64, TLS uses variant 2 where the thread pointer (FS) points to
/// the TCB at the END of the TLS block. TLS variables are accessed via
/// negative offsets from FS.
///
/// Each worker TCB needs its own TLS block to have independent thread-local
/// storage (e.g., the seL4 IPC buffer pointer).
#[cfg(all(target_arch = "x86_64", feature = "sel4"))]
#[repr(C, align(16))]
struct TlsBlock {
    /// Space for TLS data (.tdata + .tbss)
    data: [u8; TLS_BLOCK_SIZE],
    /// Self-pointer required by x86_64 TLS variant 2
    self_ptr: usize,
}

#[cfg(all(target_arch = "x86_64", feature = "sel4"))]
static mut TLS_BLOCKS: [TlsBlock; MAX_WORKERS] = [const {
    TlsBlock {
        data: [0u8; TLS_BLOCK_SIZE],
        self_ptr: 0,
    }
}; MAX_WORKERS];

// Linker-provided symbols for TLS template sections.
// These are defined in lona-vm.ld and mark the start/end of .tdata and .tbss.
#[cfg(all(target_arch = "x86_64", feature = "sel4"))]
unsafe extern "C" {
    static __tdata_start: u8;
    static __tdata_end: u8;
    static __tbss_start: u8;
    static __tbss_end: u8;
}

/// Initialize TLS on x86_64 using wrfsbase instruction.
///
/// Each worker gets its own TLS block (indexed by `worker_id`).
///
/// This function properly initializes TLS by:
/// 1. Calculating the TLS size from linker-provided symbols
/// 2. Copying the .tdata template to this worker's TLS block
/// 3. Zero-initializing the .tbss portion (already zero in static)
/// 4. Setting FS to point to the thread pointer (end of TLS block)
#[cfg(all(target_arch = "x86_64", feature = "sel4"))]
unsafe fn init_tls_x86_64(worker_id: u16) {
    unsafe {
        let idx = (worker_id as usize).min(MAX_WORKERS - 1);

        // Calculate TLS section sizes from linker symbols.
        let tdata_start = &__tdata_start as *const u8;
        let tdata_end = &__tdata_end as *const u8;
        let tbss_end = &__tbss_end as *const u8;

        let tdata_size = tdata_end.offset_from(tdata_start) as usize;
        let total_tls_size = tbss_end.offset_from(tdata_start) as usize;

        // Verify TLS fits in our static block
        if total_tls_size > TLS_BLOCK_SIZE {
            loop {
                asm!("hlt", options(nomem, nostack));
            }
        }

        // Use this worker's TLS block
        let tls_block_ptr = core::ptr::addr_of_mut!(TLS_BLOCKS[idx]);
        let tls_data_ptr = core::ptr::addr_of_mut!((*tls_block_ptr).data) as *mut u8;
        let tls_data_start = tls_data_ptr.add(TLS_BLOCK_SIZE - total_tls_size);

        // Copy .tdata template from ELF to this worker's TLS block
        if tdata_size > 0 {
            core::ptr::copy_nonoverlapping(tdata_start, tls_data_start, tdata_size);
        }

        // Set up the thread pointer (self-pointer for variant 2)
        let self_ptr_ptr = core::ptr::addr_of_mut!((*tls_block_ptr).self_ptr);
        let thread_pointer = self_ptr_ptr as usize;
        self_ptr_ptr.write(thread_pointer);

        // Set FS base register to the thread pointer
        asm!("wrfsbase {}", in(reg) thread_pointer, options(nostack, preserves_flags));
    }
}

/// Initialize UART on x86_64 (COM1 via seL4 IOPort).
#[cfg(all(target_arch = "x86_64", feature = "sel4"))]
fn init_uart_x86_64(boot_flags: BootFlags, worker_id: u16) -> Com1Uart {
    if boot_flags.has_uart() {
        // Initialize TLS first - required for seL4 syscalls
        // SAFETY: Each worker calls this once with its own worker_id,
        // getting its own TLS block. No concurrent access to the same block.
        unsafe {
            init_tls_x86_64(worker_id);
        }

        // Set up IPC buffer for seL4 syscalls
        let ipc_buffer_addr = worker_ipc_buffer(worker_id);
        // SAFETY: Memory manager has mapped IPC buffer at this address and
        // it will remain valid for the lifetime of this VM.
        unsafe {
            let ipc_buffer = &mut *(ipc_buffer_addr as *mut sel4::IpcBuffer);
            sel4::set_ipc_buffer(ipc_buffer);
        }

        // Initialize UART with IOPort capability from well-known CSpace slot
        let io_port_cap = CapSlot::IOPORT_UART.as_u64();
        // SAFETY: Memory manager has placed IOPort cap in this slot
        unsafe {
            lona_vm::uart::x86_64_init(io_port_cap);
        }
    }
    Com1Uart::new()
}

/// Print boot information to UART.
fn print_boot_info<U: Uart>(
    uart: &mut U,
    realm_id: u64,
    worker_id: u16,
    heap_start: u64,
    heap_size: u64,
    boot_flags: BootFlags,
) {
    uart.write_str("Lona VM ");
    uart.write_str(lona_vm::VERSION);
    uart.write_str(" starting\n");

    uart.write_str("  Realm ID: ");
    print_u64(uart, realm_id);
    uart.write_str("\n");

    uart.write_str("  Worker ID: ");
    print_u64(uart, u64::from(worker_id));
    uart.write_str("\n");

    if boot_flags.is_init_realm() {
        uart.write_str("  Init realm: yes\n");
    }

    uart.write_str("  Heap start: 0x");
    print_hex(uart, heap_start);
    uart.write_str("\n");

    uart.write_str("  Heap size: ");
    print_u64(uart, heap_size);
    uart.write_str(" bytes\n");

    // List embedded library contents
    uart.write_str("\nEmbedded libraries:\n");
    match TarSource::embedded() {
        Ok(source) => {
            for entry in source.entries() {
                let filename_tar = entry.filename();
                let Ok(filename) = filename_tar.as_str() else {
                    continue;
                };
                if filename.ends_with('/') {
                    continue;
                }
                uart.write_str("  ");
                uart.write_str(filename);
                uart.write_str(" (");
                print_u64(uart, entry.data().len() as u64);
                uart.write_str(" bytes)\n");
            }
        }
        Err(_) => {
            uart.write_str("  ERROR: Failed to load embedded archive\n");
        }
    }
}

fn print_u64<U: Uart>(uart: &mut U, mut n: u64) {
    if n == 0 {
        uart.write_byte(b'0');
        return;
    }
    let mut digits = [0u8; 20];
    let mut i = 0;
    while n > 0 {
        digits[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        uart.write_byte(digits[i]);
    }
}

fn print_hex<U: Uart>(uart: &mut U, mut n: u64) {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut digits = [0u8; 16];
    let mut i = 0;
    if n == 0 {
        uart.write_byte(b'0');
        return;
    }
    while n > 0 {
        digits[i] = HEX[(n & 0xF) as usize];
        n >>= 4;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        uart.write_byte(digits[i]);
    }
}

fn halt_loop() -> ! {
    loop {
        #[cfg(target_arch = "aarch64")]
        unsafe {
            asm!("wfe", options(nomem, nostack));
        }
        #[cfg(target_arch = "x86_64")]
        unsafe {
            asm!("hlt", options(nomem, nostack));
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    halt_loop()
}
