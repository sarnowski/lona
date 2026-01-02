//! Integration tests for lona-vm.

use lona_vm::platform::MockVSpace;
use lona_vm::platform::vspace_layout;
use lona_vm::{MemorySpace, Paddr, Pid, Vaddr, init};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ProcessHeader {
    pid: u64,
    status: u32,
    _pad: u32,
    heap_start: u64,
    heap_ptr: u64,
    stack_start: u64,
    stack_ptr: u64,
}

#[test]
fn test_vm_init() {
    let result = init();
    assert!(result.is_ok());
}

#[test]
fn test_vspace_layout_constants() {
    assert!(vspace_layout::NULL_GUARD < vspace_layout::GLOBAL_CONTROL);
    assert!(vspace_layout::GLOBAL_CONTROL < vspace_layout::SCHEDULER_STATE);
    assert!(vspace_layout::SCHEDULER_STATE < vspace_layout::NAMESPACE_RO);
    assert!(vspace_layout::NAMESPACE_RO < vspace_layout::NAMESPACE_RW);
    assert!(vspace_layout::NAMESPACE_RW < vspace_layout::NAMESPACE_OBJECTS);
    assert!(vspace_layout::NAMESPACE_OBJECTS < vspace_layout::ANCESTOR_CODE);
    assert!(vspace_layout::ANCESTOR_CODE < vspace_layout::LOCAL_CODE);
    assert!(vspace_layout::LOCAL_CODE < vspace_layout::PROCESS_HEAPS);
    assert!(vspace_layout::PROCESS_HEAPS < vspace_layout::SHARED_BINARY);
    assert!(vspace_layout::SHARED_BINARY < vspace_layout::CROSS_REALM_SHARED);
    assert!(vspace_layout::CROSS_REALM_SHARED < vspace_layout::DEVICE_MAPPINGS);
    assert!(vspace_layout::DEVICE_MAPPINGS < vspace_layout::KERNEL_RESERVED);
}

#[test]
fn test_mock_vspace_process_simulation() {
    let process_size: usize = 64 * 1024;
    let base = vspace_layout::PROCESS_HEAPS;
    let mut vspace = MockVSpace::new(process_size, base);

    let header_size = core::mem::size_of::<ProcessHeader>();
    let stack_start = base.add(header_size as u64);
    let heap_start = base.add(process_size as u64);

    let header = ProcessHeader {
        pid: Pid::new(1, 42).as_raw(),
        status: 1,
        _pad: 0,
        heap_start: heap_start.as_u64(),
        heap_ptr: heap_start.as_u64(),
        stack_start: stack_start.as_u64(),
        stack_ptr: stack_start.as_u64(),
    };

    vspace.write(base, header);

    let alloc_size: u64 = 256;
    let mut proc: ProcessHeader = vspace.read(base);
    proc.heap_ptr -= alloc_size;
    vspace.write(base, proc);

    let stack_frame_size: u64 = 64;
    let mut proc: ProcessHeader = vspace.read(base);
    proc.stack_ptr += stack_frame_size;
    vspace.write(base, proc);

    let final_proc: ProcessHeader = vspace.read(base);
    assert_eq!(final_proc.heap_ptr, heap_start.as_u64() - alloc_size);
    assert_eq!(
        final_proc.stack_ptr,
        stack_start.as_u64() + stack_frame_size
    );
    assert!(final_proc.heap_ptr > final_proc.stack_ptr);
}

#[test]
fn test_address_type_safety() {
    let paddr = Paddr::new(0x1000);
    let vaddr = Vaddr::new(0x1000);

    assert_eq!(paddr.as_u64(), vaddr.as_u64());

    let paddr2 = paddr.add(0x100);
    let vaddr2 = vaddr.add(0x100);
    assert_eq!(paddr2.as_u64(), 0x1100);
    assert_eq!(vaddr2.as_u64(), 0x1100);
}

#[test]
fn test_pid_structure() {
    let pid = Pid::new(5, 42);

    assert_eq!(pid.realm_id(), 5);
    assert_eq!(pid.local_id(), 42);

    let raw = pid.as_raw();
    let restored = Pid::from_raw(raw);
    assert_eq!(pid, restored);

    let same_realm = Pid::new(5, 100);
    let diff_realm = Pid::new(6, 42);
    assert!(pid.same_realm(same_realm));
    assert!(!pid.same_realm(diff_realm));
}
