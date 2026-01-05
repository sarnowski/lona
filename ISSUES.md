# Known Issues

## x86_64 E2E Tests Hang

**Status:** Open
**Affects:** E2E tests on x86_64 only
**Does NOT affect:** Release REPL on x86_64, all aarch64 builds

### Symptoms

When running `make x86_64-test`, the VM starts but hangs during UART initialization. No output is produced from the VM. The hang occurs specifically in `sel4::set_ipc_buffer()`.

The release build (`make run-x86_64`) works correctly and boots into the REPL.

### Root Cause Analysis

The issue is related to Thread Local Storage (TLS) initialization on x86_64.

#### Background

On x86_64, the sel4 crate uses `#[thread_local]` variables to store the IPC buffer pointer. When `sel4::set_ipc_buffer()` is called, it writes to this thread-local variable.

Thread-local access on x86_64 works via:
1. The FS segment register points to the Thread Control Block (TCB)
2. Thread-local variables are accessed as `fs:NEGATIVE_OFFSET`
3. The linker determines offsets based on the ELF's `.tdata`/`.tbss` sections

#### What We Do

We initialize TLS manually in `init_tls_x86_64()`:
```rust
static mut TLS_REGION: TlsRegion = TlsRegion {
    tls_data: [0u8; 64],
    self_ptr: 0,
};

unsafe fn init_tls_x86_64() {
    let thread_pointer = &TLS_REGION.self_ptr as *const _ as usize;
    TLS_REGION.self_ptr = thread_pointer;
    asm!("wrfsbase {}", in(reg) thread_pointer);
}
```

#### The Problem

Our `TLS_REGION` is a separate static variable, NOT the ELF's TLS segment. When we set FS to point to our `TLS_REGION.self_ptr`, the sel4 crate's thread-local variables end up at incorrect addresses because:

1. The compiler placed sel4's `#[thread_local]` variables in the ELF's TLS segment
2. The linker calculated offsets relative to that segment
3. We point FS to our own structure instead
4. When sel4 accesses `fs:OFFSET`, it reads/writes to wrong memory

#### Why Release Works But Debug Doesn't

Hypothesis (unconfirmed):
- In release builds, aggressive optimization may inline and eliminate actual TLS accesses
- Or the release binary's TLS layout happens to be compatible by accident
- Debug builds preserve all TLS accesses, exposing the mismatch

### Attempted Fixes

1. **Increased TLS_REGION size** (64 → 4096 bytes) - No effect
2. **Disabled TLS at target level** (`has-thread-local: false`) - Still hangs
3. **Moved TLS init earlier** (before any function calls) - No effect
4. **Removed TLS init entirely** - Still hangs (sel4 crate still uses TLS internally)

### Potential Solutions

1. **Proper ELF TLS Segment Setup**: Find the ELF's TLS segment at runtime and point FS to it correctly. This requires parsing the ELF headers or having the memory manager pass the TLS info.

2. **Patch sel4 Crate**: Modify the sel4 crate to use a regular static instead of `#[thread_local]` for the IPC buffer pointer.

3. **Custom sel4 Build**: Build sel4 crate with a feature flag that disables TLS usage.

4. **Linker Script**: Use a custom linker script to place our TLS_REGION at the correct offset relative to where the linker expects the TCB.

### Other Issues Fixed During Investigation

#### Makefile truncate Corruption

The Makefile used `truncate -s 1M` which SHRINKS files larger than 1MB. Debug builds are ~4MB, causing binary corruption.

**Fix:** Changed to `truncate -s ">1M"` which only extends files smaller than 1M, leaving larger files unchanged.

#### Boot Args Clobbered by Debug Prints

On x86_64, boot arguments are passed in registers (RDI, RSI, RDX, RCX, R8). Any function call before `read_boot_args()` clobbers these registers.

Adding `sel4::debug_println!()` before reading boot args caused heap_start and heap_size to be 0.

**Fix:** Ensure `read_boot_args()` is called first, before any other function calls.

### Workaround

Skip x86_64 E2E tests for now. The release REPL works correctly on both architectures.
