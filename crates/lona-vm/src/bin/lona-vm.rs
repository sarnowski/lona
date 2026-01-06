// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Lona VM Entry Point
//!
//! This is the entry point for all Lona realms. The Lona Memory Manager
//! starts a TCB with PC pointing here after creating a realm.

#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

// Global allocator for heap allocations (Vec, etc.)
// Uses a simple bump allocator with a fixed-size buffer.
mod allocator {
    use core::alloc::{GlobalAlloc, Layout};
    use core::cell::UnsafeCell;
    use core::ptr;
    use core::sync::atomic::{AtomicUsize, Ordering};

    /// Size of the allocation buffer (64 KB should be plenty for bytecode chunks).
    const ALLOC_SIZE: usize = 64 * 1024;

    /// Simple bump allocator for no_std environments.
    pub struct BumpAllocator {
        buffer: UnsafeCell<[u8; ALLOC_SIZE]>,
        next: AtomicUsize,
    }

    // SAFETY: Single-threaded environment, no concurrent access.
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
use lona_vm::process::pool::ProcessPool;
use lona_vm::process::{INITIAL_OLD_HEAP_SIZE, INITIAL_YOUNG_HEAP_SIZE, Process};
#[cfg(not(feature = "e2e-test"))]
use lona_vm::repl;
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
/// Boot arguments are passed in registers - must read them before any function calls.
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // Read boot arguments from registers immediately - MUST be first, before any function calls
    let (realm_id, worker_id, heap_start, heap_size, flags) = read_boot_args();

    let boot_flags = BootFlags::new(flags);

    // Initialize UART based on platform
    #[cfg(target_arch = "aarch64")]
    let mut uart = init_uart_aarch64(boot_flags);

    #[cfg(all(target_arch = "x86_64", feature = "sel4"))]
    let mut uart = init_uart_x86_64(boot_flags, worker_id);

    // Print boot info if UART is available
    if boot_flags.has_uart() {
        print_boot_info(
            &mut uart, realm_id, worker_id, heap_start, heap_size, boot_flags,
        );
    }

    // Create process pool from boot-allocated heap memory
    let mut pool = ProcessPool::new(Vaddr::new(heap_start), heap_size as usize);

    // Allocate memory for REPL process (young heap + old heap)
    let (young_base, old_base) = pool
        .allocate_process_memory(INITIAL_YOUNG_HEAP_SIZE, INITIAL_OLD_HEAP_SIZE)
        .expect("failed to allocate REPL process memory");

    // Create REPL process with BEAM-style memory layout
    let mut process = Process::new(
        1, // PID 1 for REPL
        young_base,
        INITIAL_YOUNG_HEAP_SIZE,
        old_base,
        INITIAL_OLD_HEAP_SIZE,
    );

    let mut vspace = Sel4VSpace;

    // Run E2E tests when feature is enabled, otherwise start REPL
    #[cfg(feature = "e2e-test")]
    {
        e2e::run_all_tests(&mut process, &mut vspace, &mut uart);
        halt_loop()
    }

    #[cfg(not(feature = "e2e-test"))]
    {
        if boot_flags.has_uart() {
            uart.write_line("\nStarting REPL...\n");
        }
        repl::run(&mut process, &mut vspace, &mut uart)
    }
}

/// Initialize UART on aarch64 (PL011 MMIO).
#[cfg(target_arch = "aarch64")]
fn init_uart_aarch64(boot_flags: BootFlags) -> Pl011Uart {
    if boot_flags.has_uart() {
        // SAFETY: Memory manager has mapped UART at UART_VADDR
        unsafe {
            lona_vm::uart::aarch64_init(UART_VADDR as *mut u8);
        }
    }
    Pl011Uart::new()
}

/// Maximum TLS block size in bytes (generous for debug builds).
/// The sel4 crate needs ~16 bytes for IPC buffer pointer, but debug builds
/// may have additional TLS variables from debug assertions and other code.
#[cfg(all(target_arch = "x86_64", feature = "sel4"))]
const TLS_BLOCK_SIZE: usize = 4096;

/// Static TLS block for x86_64.
///
/// On x86_64, TLS uses variant 2 where the thread pointer (FS) points to
/// the TCB at the END of the TLS block. TLS variables are accessed via
/// negative offsets from FS.
///
/// Layout:
/// ```text
/// +----------------------------------+
/// | .tdata (copied from ELF)         | <- TLS variables (initialized)
/// +----------------------------------+
/// | .tbss (zero-initialized)         | <- TLS variables (uninitialized)
/// +----------------------------------+
/// | self_ptr (points to itself)      | <- FS points here
/// +----------------------------------+
/// ```
#[cfg(all(target_arch = "x86_64", feature = "sel4"))]
#[repr(C, align(16))]
struct TlsBlock {
    /// Space for TLS data (.tdata + .tbss)
    data: [u8; TLS_BLOCK_SIZE],
    /// Self-pointer required by x86_64 TLS variant 2
    self_ptr: usize,
}

#[cfg(all(target_arch = "x86_64", feature = "sel4"))]
static mut TLS_BLOCK: TlsBlock = TlsBlock {
    data: [0u8; TLS_BLOCK_SIZE],
    self_ptr: 0,
};

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
/// This function properly initializes TLS by:
/// 1. Calculating the TLS size from linker-provided symbols
/// 2. Copying the .tdata template to the TLS block
/// 3. Zero-initializing the .tbss portion (already zero in static)
/// 4. Setting FS to point to the thread pointer (end of TLS block)
#[cfg(all(target_arch = "x86_64", feature = "sel4"))]
unsafe fn init_tls_x86_64() {
    unsafe {
        // Calculate TLS section sizes from linker symbols.
        // Use tbss_end - tdata_start to include any alignment padding between sections.
        let tdata_start = &__tdata_start as *const u8;
        let tdata_end = &__tdata_end as *const u8;
        let tbss_end = &__tbss_end as *const u8;

        let tdata_size = tdata_end.offset_from(tdata_start) as usize;
        let total_tls_size = tbss_end.offset_from(tdata_start) as usize;

        // Verify TLS fits in our static block
        // Note: In production, this should never fail as TLS_BLOCK_SIZE is generous
        if total_tls_size > TLS_BLOCK_SIZE {
            // TLS too large - halt permanently. Single HLT can return after interrupt.
            loop {
                asm!("hlt", options(nomem, nostack));
            }
        }

        // Calculate where TLS data should start in our block.
        // For variant 2, TLS data is placed BEFORE the thread pointer.
        // The thread pointer (FS) will point to the self_ptr field.
        //
        // Our block layout:
        //   TLS_BLOCK.data[TLS_BLOCK_SIZE - total_tls_size .. TLS_BLOCK_SIZE] = TLS data
        //   TLS_BLOCK.self_ptr = thread pointer (FS points here)
        //
        // TLS variables have negative offsets from FS, so:
        //   fs:[offset] where offset is negative
        //
        // The linker computes offsets as if TLS data ends right before the TCB.
        let tls_block_ptr = core::ptr::addr_of_mut!(TLS_BLOCK);
        let tls_data_ptr = core::ptr::addr_of_mut!((*tls_block_ptr).data) as *mut u8;
        let tls_data_start = tls_data_ptr.add(TLS_BLOCK_SIZE - total_tls_size);

        // Copy .tdata template from ELF to our TLS block
        if tdata_size > 0 {
            core::ptr::copy_nonoverlapping(tdata_start, tls_data_start, tdata_size);
        }

        // .tbss is already zero-initialized in the static TLS_BLOCK

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
        // SAFETY: Single-threaded, called once at startup
        unsafe {
            init_tls_x86_64();
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

/// Read boot arguments from CPU registers.
/// Must be called at the very start before registers are clobbered.
#[inline(always)]
fn read_boot_args() -> (u64, u16, u64, u64, u64) {
    let realm_id: u64;
    let worker_id: u64;
    let heap_start: u64;
    let heap_size: u64;
    let flags: u64;

    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!(
            "", // Empty asm block - just read the registers
            out("x0") realm_id,
            out("x1") worker_id,
            out("x2") heap_start,
            out("x3") heap_size,
            out("x4") flags,
            options(nomem, nostack, preserves_flags)
        );
    }

    #[cfg(target_arch = "x86_64")]
    unsafe {
        asm!(
            "",
            out("rdi") realm_id,
            out("rsi") worker_id,
            out("rdx") heap_start,
            out("rcx") heap_size,
            out("r8") flags,
            options(nomem, nostack, preserves_flags)
        );
    }

    // Worker ID is passed as u64 but is guaranteed to fit in u16 (max 256 workers)
    // Mask to u16 range to ensure no truncation issues
    let worker_id = (worker_id & 0xFFFF) as u16;

    (realm_id, worker_id, heap_start, heap_size, flags)
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
