# System Architecture

This document covers Lona's system architecture built on seL4: the Lona Memory Manager, realms, threads, bootstrapping, and IPC.

## Architectural Separation

A key design decision: **separate the resource authority from the Lona VM**.

### The Problem with Conflation

If one component handles both resource management AND runs the Lona VM:

```
Conflated Design (NOT what we do):
┌─────────────────────────────────────────────────────────────────────┐
│                       MONOLITHIC TASK                               │
│                                                                     │
│  Mixed responsibilities:                                           │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ 1. Memory Authority (Untyped, fault handling)               │   │
│  │ 2. Lona VM (bytecode interpreter, GC, scheduler)            │   │
│  │ 3. Init/Supervisor Logic (spawning realms)                  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  Problems:                                                         │
│  - Large Trusted Computing Base (TCB)                              │
│  - Bug in Lona VM could compromise resource management            │
│  - Complex, hard to audit                                          │
└─────────────────────────────────────────────────────────────────────┘
```

### The Solution: Two Binaries

```
Separated Design:
┌─────────────────────────────────────────────────────────────────────┐
│  Crate: lona-abi                                                    │
│  - IPC message types                                               │
│  - Protocol definitions                                            │
│  - Shared constants                                                │
└───────────────────────────┬─────────────────────────────────────────┘
                            │
              ┌─────────────┴─────────────┐
              ▼                           ▼
┌──────────────────────────┐   ┌──────────────────────────────────────┐
│  Lona Memory Manager     │   │  Lona VM                             │
│                          │   │                                      │
│  - Resource authority    │   │  - Bytecode interpreter              │
│  - Untyped pool          │   │  - Process multiplexer               │
│  - Fault handler         │   │  - Per-process GC                    │
│  - Realm lifecycle       │   │  - Message passing                   │
│  - Policy enforcement    │   │  - Scheduler                         │
│                          │   │                                      │
│  Small, auditable        │   │  Complex, but isolated               │
│  NO Lonala code          │   │  Mapped into every realm             │
└──────────────────────────┘   └──────────────────────────────────────┘
```

### Benefits

| Aspect | Benefit |
|--------|---------|
| **Smaller TCB** | Lona Memory Manager is minimal, auditable |
| **Fault isolation** | Bug in Lona VM can't corrupt the Memory Manager |
| **Restartability** | Memory Manager could restart crashed realms |
| **Uniformity** | All realms run identical VM code |
| **Clear separation** | No risk of accidentally sharing code |

---

## Threads in seL4

### TCB (Thread Control Block)

A thread is represented by a **TCB** kernel object:

```
TCB (Thread Control Block)
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  CPU State (saved when not running):                               │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  Program Counter (PC/RIP) - where to execute next           │   │
│  │  Stack Pointer (SP/RSP)   - top of stack                    │   │
│  │  General registers        - r0-r30 / rax,rbx,etc            │   │
│  │  Flags/Status register                                      │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  Bindings:                                                         │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  VSpace cap    - which address space this thread uses       │   │
│  │  CSpace cap    - which capability space for syscalls        │   │
│  │  Fault endpoint - where to send faults                      │   │
│  │  SchedContext  - CPU time budget (MCS scheduler)            │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

A thread doesn't "own" a VSpace - it's **bound** to one. Multiple threads can share a VSpace.

### Starting a Thread

There's no "jump into" an address space. You **configure** the TCB and **resume** it:

```
1. Retype Untyped → TCB
   seL4_Untyped_Retype(untyped, seL4_TCBObject, ...)

2. Configure the TCB
   seL4_TCB_Configure(
       tcb_cap,
       fault_ep,          // Where faults go
       cspace_cap,        // Thread's capability space
       cspace_root_data,
       vspace_cap,        // Thread's address space
       vspace_root_data,
       ipc_buffer_addr,
       ipc_buffer_frame_cap
   )

3. Set initial register state
   seL4_TCB_WriteRegisters(
       tcb_cap,
       resume: false,     // Don't start yet
       regs: {
           pc: 0x4000_0000,   // Entry point (must be mapped!)
           sp: 0x5000_0000,   // Stack top (must be mapped!)
       }
   )

4. Bind scheduling context (MCS)
   seL4_SchedContext_Bind(sched_context_cap, tcb_cap)

5. Resume the thread
   seL4_TCB_Resume(tcb_cap)

   → Thread starts executing at PC in its VSpace
```

### Entry Points

The PC register is just an address. You can set it to any mapped, executable address:

```
Binary in Physical Memory:

0x0010_0000 ┌────────────────────────────────────┐
            │ _start:                            │ ← ELF default entry
            │   ; CRT initialization             │
            │   call main                        │
            │                                    │
0x0010_0100 │ main:                              │ ← One possible entry
            │   ; Application code               │
            │                                    │
0x0010_2000 │ realm_entry:                       │ ← Another possible entry
            │   ; Different entry point          │
            └────────────────────────────────────┘

When starting a thread, you CHOOSE which function to call by setting PC.
```

---

## Bootstrapping Sequence

### Boot Image Contents

```
BOOT IMAGE:
┌─────────────────────────────────────────────────────────────────────┐
│  seL4 Kernel                                                        │
├─────────────────────────────────────────────────────────────────────┤
│  Lona Memory Manager (ELF)  ← Loaded by kernel, becomes root task  │
├─────────────────────────────────────────────────────────────────────┤
│  Lona VM (ELF)              ← Boot module, mapped into realms      │
│    └── lonalib.tar (embedded) ← Standard library source code       │
└─────────────────────────────────────────────────────────────────────┘
```

The Lona VM binary embeds the standard library as a USTAR tar archive (`lonalib.tar`) containing Lonala source files. Source code is compiled on demand at runtime. See [Library Loading](../development/library-loading.md) for details.

### Physical Memory After Boot

```
0x0010_0000  ┌─────────────────────────┐
             │ Lona Memory Manager     │ ← Kernel mapped this
             │ Entry: 0x0010_0000      │
             └─────────────────────────┘
0x0020_0000  ┌─────────────────────────┐
             │ Lona VM code            │ ← Boot module
             │ Entry: 0x0020_0000      │
             │ (includes lonalib.tar)  │
             └─────────────────────────┘
0x0100_0000  ┌─────────────────────────┐
             │ Free RAM (Untyped)      │
             └─────────────────────────┘
```

### Lona Memory Manager Startup

The kernel starts the Lona Memory Manager:

```
Lona Memory Manager's Initial State:
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  VSpace: Memory Manager code mapped, executing                     │
│                                                                     │
│  CSpace:                                                           │
│    Slot 1: TCB cap (own thread)                                    │
│    Slot 2: CSpace cap (own cspace)                                 │
│    Slot 3: VSpace cap (own vspace)                                 │
│    Slot 4-N: Untyped caps (all free physical memory)               │
│    Slot N+1: Frame caps for boot modules                           │
│                                                                     │
│  Bootinfo: describes where everything is                           │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Creating Init Realm

Lona Memory Manager creates the first Lonala realm:

```
Step 1: Create kernel objects from Untyped
────────────────────────────────────────────────────────────────────────

┌─────────────┐
│   Untyped   │ ──retype──▶ VSpace (init realm's address space)
│   Pool      │ ──retype──▶ CSpace (init realm's capability space)
│             │ ──retype──▶ TCB (init realm's first worker)
│             │ ──retype──▶ SchedContext (CPU budget)
│             │ ──retype──▶ Endpoint (init realm's IPC endpoint)
│             │ ──retype──▶ Page tables (PUD, PD, PT as needed)
│             │ ──retype──▶ Frames (for stack, IPC buffer)
└─────────────┘

Memory Manager stores: ep_init → realm_id: 1 (for caller identification)


Step 2: Map Lona VM code into init realm's VSpace
────────────────────────────────────────────────────────────────────────

SAME physical frames, mapped into TWO VSpaces:

Memory Manager VSpace      Init Realm VSpace
┌─────────────────┐         ┌─────────────────┐
│ VM code @ 0x... │         │ VM code @ 0x... │
└────────┬────────┘         └────────┬────────┘
         │                           │
         └───────────┬───────────────┘
                     │
                     ▼
           ┌─────────────────┐
           │ Physical Frames │
           │ (Lona VM)       │
           └─────────────────┘

Same physical memory, mapped at the SAME virtual addresses in all realms.
This ensures pointers within the code remain valid across all VSpaces.
Mapped read-only + execute.


Step 3: Map standard library (embedded in VM)
────────────────────────────────────────────────────────────────────────

The Lona VM binary includes the embedded lonalib.tar containing Lonala
source files. No separate bytecode mapping is needed - source is compiled
on demand at runtime. See [Library Loading](../development/library-loading.md).


Step 4: Configure TCB
────────────────────────────────────────────────────────────────────────

seL4_TCB_Configure(
    init_tcb_cap,
    ep_init_cap,            // Fault endpoint: Memory Manager's ep_init
    init_cspace_cap,        // Init realm's capability space
    guard_data,
    init_vspace_cap,        // Init realm's address space
    0,
    ipc_buffer_vaddr,       // IPC buffer in worker stacks region
    ipc_buffer_frame_cap
)

// ep_init is OWNED by Memory Manager, USED BY init realm:
// - Memory Manager created ep_init and listens on it
// - Init realm's TCB sends faults to ep_init
// - Memory Manager receives on ep_init, knows it's from init realm

Also install Send cap to ep_init in init realm's CSpace for IPC requests.


Step 5: Set initial registers
────────────────────────────────────────────────────────────────────────

seL4_UserContext regs = {
    .pc = VM_ENTRY,         // Lona VM entry point
    .sp = stack_top,        // Stack pointer

    // Pass arguments in registers (ABI-dependent):
    .r0 = heap_start,       // Where heap begins
    .r1 = heap_size,        // Initial heap size
    // Note: Library source is embedded in VM binary (lonalib.tar)
    // and compiled on demand - no separate bytecode pointer needed
};

seL4_TCB_WriteRegisters(init_tcb_cap, false, 0, 4, &regs);


Step 6: Start the worker
────────────────────────────────────────────────────────────────────────

seL4_SchedContext_Bind(sched_context_cap, init_tcb_cap);
seL4_TCB_Resume(init_tcb_cap);

→ TCB starts executing Lona VM
→ VM loads source from embedded lonalib.tar
→ VM compiles and runs init.lona
→ Init realm is now running!
```

---

## Workers and Processes

### Terminology

| Term | seL4 Object | What It Is |
|------|-------------|------------|
| **Realm** | VSpace + CSpace + SchedContext | Security boundary |
| **Worker** | TCB | Kernel-scheduled thread, runs Lona VM |
| **Process** | (none - pure userspace) | Lonala lightweight process |

### Execution Model

```
REALM
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  VSpace: contains Lona VM code + bytecode + heap                    │
│  CSpace: capabilities for IPC, resources                            │
│  TCB(s): one or more workers                                        │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  Worker TCB 1          Worker TCB 2          Worker TCB N   │    │
│  │  (Lona VM)             (Lona VM)             (Lona VM)      │    │
│  │       │                     │                     │         │    │
│  │       ▼                     ▼                     ▼         │    │
│  │  ┌─────────┐           ┌─────────┐           ┌─────────┐    │    │
│  │  │ Proc A  │           │ Proc D  │           │ Proc G  │    │    │
│  │  │ Proc B  │           │ Proc E  │           │ Proc H  │    │    │
│  │  │ Proc C  │           │ Proc F  │           │  ...    │    │    │
│  │  └─────────┘           └─────────┘           └─────────┘    │    │
│  │                                                             │    │
│  │  Lonala processes multiplexed across workers                │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

TCBs are the kernel's scheduling unit. Lonala processes are the VM's scheduling unit - two levels of multiplexing.

### Worker Count

**Decision: 1 worker per CPU core**

Each realm creates one worker (TCB) per available CPU core. This enables parallel execution within a realm while keeping the Lona VM scheduler simple.

- Each worker runs on a dedicated CPU core
- Workers within a realm share the VSpace (address space)
- MCS scheduler enforces CPU budget across realms
- Future: work stealing between workers for load balancing

---

## IPC and Capabilities

### Per-Realm Endpoints (Identity Model)

Each realm gets its own dedicated Endpoint object for communicating with the Lona Memory Manager. This provides **unforgeable identity** - the endpoint object itself identifies the caller, not a badge that could be spoofed.

```
PER-REALM ENDPOINT MODEL
════════════════════════════════════════════════════════════════════════

Lona Memory Manager creates a SEPARATE endpoint for each realm:

┌─────────────────────────────────────────────────────────────────────┐
│  LONA MEMORY MANAGER                                                │
│                                                                     │
│  On realm creation:                                                 │
│    1. Create Endpoint object for this realm                         │
│    2. Store mapping: endpoint → realm_id                            │
│    3. Give realm a Send capability to its endpoint                  │
│                                                                     │
│  Endpoint objects in Memory Manager's CSpace:                       │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  ep_init      → realm_id: 1 (init realm)                    │    │
│  │  ep_app_a     → realm_id: 2 (app realm A)                   │    │
│  │  ep_app_b     → realm_id: 3 (app realm B)                   │    │
│  │  ep_driver    → realm_id: 4 (driver realm)                  │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                     │
│  Identity = which endpoint received the message (unforgeable)       │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

Each realm has capability to its OWN endpoint only:

Init Realm:           App Realm A:          App Realm B:
┌───────────────┐     ┌───────────────┐     ┌───────────────┐
│ cap: ep_init  │     │ cap: ep_app_a │     │ cap: ep_app_b │
│ (Send only)   │     │ (Send only)   │     │ (Send only)   │
└───────────────┘     └───────────────┘     └───────────────┘
```

### Why Not Badges?

Badges are metadata embedded in capabilities. If a realm could pass or mint capabilities, it could impersonate other realms:

```
BADGE PROBLEM (why we don't use this):
────────────────────────────────────────────────────────────────────────

If Init Realm passes its badged cap to App Realm A:
  → App A can call Memory Manager with Init's badge
  → Memory Manager thinks it's Init Realm
  → App A can request realm creation, etc.

Per-realm endpoints avoid this:
  → Even if Init passes its cap to App A
  → Memory Manager knows ep_init = Init's endpoint
  → Init explicitly delegated its authority (acceptable)
  → App A cannot impersonate other realms
```

### Fault Handling

Each realm's TCBs have their realm-specific fault endpoint configured:

```
FAULT HANDLING
════════════════════════════════════════════════════════════════════════

LONA MEMORY MANAGER
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  Waits on all realm endpoints (multiplexed receive)             │
│                                                                 │
│  loop {                                                         │
│      (endpoint, fault_info) = wait_any_endpoint();              │
│      realm_id = endpoint_to_realm[endpoint];                    │
│      handle_fault(realm_id, fault_info);                        │
│      reply(endpoint);  // resume faulting thread                │
│  }                                                              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                                  ▲
                                  │ fault IPC (per-realm endpoints)
    ┌─────────────────────────────┼──────────────────────────────┐
    │                             │                              │
    ▼                             ▼                              ▼
┌────────────┐            ┌────────────┐             ┌────────────┐
│Init Realm  │            │ App Realm  │             │Driver Realm│
│            │            │            │             │            │
│TCB config: │            │TCB config: │             │TCB config: │
│fault_ep=   │            │fault_ep=   │             │fault_ep=   │
│ep_init     │            │ep_app_a    │             │ep_driver   │
└────────────┘            └────────────┘             └────────────┘

Page fault in any realm → seL4 sends fault IPC to realm's endpoint
Memory Manager identifies realm by which endpoint received the fault
Memory Manager handles it, replies, thread resumes
```

### Capability Distribution

```
CAPABILITY DISTRIBUTION
════════════════════════════════════════════════════════════════════════

Every realm receives (created by Lona Memory Manager):
┌─────────────────────────────────────────────────────────────────┐
│  IPC capabilities:                                              │
│  - request_ep: Send cap to this realm's endpoint (for requests) │
│                                                                 │
│  TCBs configured with:                                          │
│  - fault_ep: this realm's endpoint (for page faults)            │
│  - vspace: this realm's VSpace                                  │
│  - cspace: this realm's CSpace                                  │
│  - sched_context: this realm's CPU budget                       │
│                                                                 │
│  IPC buffer:                                                    │
│  - Frame mapped in worker stacks region                         │
│  - Required by seL4 for all IPC operations                      │
└─────────────────────────────────────────────────────────────────┘

Init realm additionally receives:
┌─────────────────────────────────────────────────────────────────┐
│  - Elevated permissions in IPC protocol (checked by Memory Mgr) │
│    (realm creation requests accepted from init realm only)      │
└─────────────────────────────────────────────────────────────────┘

Driver realms additionally receive:
┌─────────────────────────────────────────────────────────────────┐
│  - device_frame_caps: to map MMIO regions                       │
│  - irq_handler_caps: to handle interrupts                       │
│  - notification_caps: for interrupt delivery                    │
└─────────────────────────────────────────────────────────────────┘
```

### IPC Protocol

Realms communicate with Lona Memory Manager via their dedicated endpoint:

```
IPC REQUEST FLOW
════════════════════════════════════════════════════════════════════════

Realm wants to request memory:

1. Realm: seL4_Call(request_ep, msg{type: ALLOC_PAGES, count: 10})
       │
       ▼
2. Memory Manager receives on realm's endpoint
   - Knows caller identity (endpoint → realm_id mapping)
   - Validates request against realm's policy/quota
       │
       ▼
3. Memory Manager: allocates pages, maps into realm's VSpace
       │
       ▼
4. Memory Manager: seL4_Reply(msg{status: OK, vaddr: 0x...})
       │
       ▼
5. Realm: receives reply, continues execution
```

---

## Fault Handling Policy

The Lona Memory Manager handles all page faults from all realms. This requires careful policy to prevent DoS attacks and ensure correct memory mapping.

### Rate Limiting

To prevent malicious or buggy realms from overwhelming the Memory Manager with faults:

```
FAULT RATE LIMITING (Per-Realm)
════════════════════════════════════════════════════════════════════════

Memory Manager tracks fault rate per realm:

struct RealmFaultState {
    fault_count: u32,
    window_start: Timestamp,
    suspended: bool,
}

const MAX_FAULTS_PER_WINDOW: u32 = 1000;  // Configurable
const FAULT_WINDOW_MS: u64 = 1000;        // 1 second window

On fault from realm:
────────────────────────────────────────────────────────────────────────

fn handle_fault(realm_id, fault_info):
    realm = get_realm_state(realm_id)

    // Reset window if expired
    if now() - realm.window_start > FAULT_WINDOW_MS:
        realm.fault_count = 0
        realm.window_start = now()

    // Check rate limit
    if realm.fault_count >= MAX_FAULTS_PER_WINDOW:
        // Realm is misbehaving - suspend it
        realm.suspended = true
        notify_supervisor(realm_id, FAULT_RATE_EXCEEDED)
        // Don't reply - realm stays blocked
        return

    realm.fault_count += 1

    // ... proceed with normal fault handling
```

Suspended realms can be resumed by their supervisor after investigation.

### Region Table

Lona Memory Manager maintains a region table for each realm, defining valid memory regions:

```
PER-REALM REGION TABLE
════════════════════════════════════════════════════════════════════════

struct RealmMemoryMap {
    realm_id: RealmId,
    regions: Vec<Region>,
    ancestors: Vec<AncestorFrameMap>,

    // Quota tracking
    max_physical_pages: usize,
    used_physical_pages: usize,
}

struct Region {
    va_start: VAddr,
    va_end: VAddr,
    region_type: RegionType,
    permissions: Permissions,  // RO, RW, RX
}

enum RegionType {
    SharedCode,                    // Lona VM, core lib
    Inherited { ancestor: u8 },    // Parent's code/binary region
    LocalCode,                     // This realm's code region
    LocalBinary,                   // This realm's binary heap
    ProcessPool,                   // Dynamic process segments
    WorkerStacks,                  // TCB stacks + IPC buffers
    MMIO { device_id: u32 },       // Device mappings (driver realms)
    Guard,                         // NEVER map (violation = terminate)
}

enum Permissions {
    RO,   // Read-only (inherited regions, shared code)
    RW,   // Read-write (heap, stack, local data)
    RX,   // Read-execute (code regions)
}
```

### Permission Enforcement

Every fault is validated against the region table:

```
STRICT FAULT VALIDATION
════════════════════════════════════════════════════════════════════════

fn validate_and_handle_fault(realm_id, fault_addr, fault_type):
    realm = get_realm_state(realm_id)

    // 1. Find region containing fault address
    region = realm.regions.find(|r| r.contains(fault_addr))

    if region is None:
        // Access outside any valid region
        terminate_realm(realm_id, INVALID_MEMORY_ACCESS)
        return

    // 2. Check region type
    if region.type == Guard:
        // Touched a guard page - stack overflow or buffer overrun
        terminate_realm(realm_id, GUARD_PAGE_VIOLATION)
        return

    // 3. Check permissions match fault type
    match (fault_type, region.permissions):
        (Write, RO) | (Write, RX):
            terminate_realm(realm_id, WRITE_TO_READONLY)
            return
        (Execute, RO) | (Execute, RW):
            terminate_realm(realm_id, EXECUTE_NON_EXECUTABLE)
            return
        _ => ()  // OK

    // 4. Check quota
    if realm.used_physical_pages >= realm.max_physical_pages:
        notify_realm(realm_id, OUT_OF_MEMORY)
        // Don't map, realm must free memory or request more
        return

    // 5. Resolve frame and map
    frame_cap = resolve_frame(realm, region, fault_addr)
    map_frame(frame_cap, realm.vspace, fault_addr, region.permissions)
    realm.used_physical_pages += 1

    // 6. Reply to resume faulting thread
    reply_to_fault()
```

### Frame Resolution for Inherited Regions

Inherited regions use lazy mapping - frames are only mapped when first accessed:

```
LAZY INHERITED FRAME MAPPING
════════════════════════════════════════════════════════════════════════

When creating a child realm, Memory Manager records frame cap references
for each ancestor's regions (but doesn't map them yet):

struct AncestorFrameMap {
    ancestor_level: u8,           // 0 = root, 1 = init, etc.
    code_region_frames: Vec<FrameCap>,
    binary_region_frames: Vec<FrameCap>,
}

On fault in inherited region:
────────────────────────────────────────────────────────────────────────

fn resolve_frame(realm, region, fault_addr) -> FrameCap:
    match region.type:
        SharedCode =>
            // Same frames for all realms, already known
            shared_code_frames.get_for_addr(fault_addr)

        Inherited { ancestor } =>
            // Look up frame from ancestor's frame list
            ancestor_map = realm.ancestors[ancestor]
            offset = fault_addr - region.va_start
            page_index = offset / PAGE_SIZE

            if region is code_subregion:
                ancestor_map.code_region_frames[page_index]
            else:
                ancestor_map.binary_region_frames[page_index]

        LocalCode | LocalBinary | ProcessPool | WorkerStacks =>
            // Allocate fresh frame from realm's untyped budget
            allocate_frame(realm.untyped_pool)

        MMIO { device_id } =>
            // Return device frame cap (pre-allocated for driver realms)
            realm.device_frames[device_id].get_for_addr(fault_addr)
```

### Inherited Region Mutability

Inherited regions are **live-shared**, not snapshots:

```
LIVE SHARING SEMANTICS
════════════════════════════════════════════════════════════════════════

When parent updates a var binding:
    1. Parent writes new value to its code region (RW for parent)
    2. Child has same physical frames mapped (RO for child)
    3. Child immediately sees the new value

This enables:
    - Hot code reloading (parent updates, children see new code)
    - Shared configuration updates
    - Dynamic system evolution

Children can SHADOW inherited vars:
    - Child defines same var name in its local region
    - Local binding takes precedence over inherited
    - Original inherited var unchanged
```

**Atomicity guarantee**: Var binding updates are atomic. A child reading a var always sees either the old value or the new value, never a partially-updated (torn) state. This is achieved through single-pointer updates to the var's root binding.

---

## Lona VM Entry Point

The Lona VM binary has its own entry point, separate from the Lona Memory Manager:

```rust
// Lona VM entry point (conceptual)

#[no_mangle]
pub extern "C" fn realm_entry(
    heap_start: *mut u8,
    heap_size: usize,
) -> ! {
    // Initialize VM state
    let mut vm = VM::new(heap_start, heap_size);

    // Load standard library from embedded lonalib.tar
    // Source is compiled on demand at runtime
    let source = TarSource::embedded();
    vm.load_namespace(&source, "lona.core");
    vm.load_namespace(&source, "lona.init");

    // Start the init process
    vm.run_init();

    // Should not return
    loop {}
}
```

The Lona Memory Manager knows this entry point address (from ELF header or fixed convention) and sets PC accordingly when starting realm threads. The standard library source is embedded in the VM binary via `lonalib.tar`.

---

## VM Scheduler Loop

Within a realm, the Lona VM multiplexes Lonala processes:

```
loop {
    // Pick a runnable process
    proc = scheduler.next()

    // Execute some bytecode (reduction-counted)
    result = interpret(proc, MAX_REDUCTIONS)

    match result {
        Yielded => scheduler.enqueue(proc),
        Blocked(mailbox) => scheduler.wait(proc, mailbox),
        Exited(reason) => cleanup(proc),
    }
}
```

This is a cooperative/preemptive hybrid: processes yield after N reductions, but the kernel preempts workers via MCS scheduling for CPU budgets across realms.

---

## Resource Management

Realms are created with resource policies that define their CPU and memory limits. These policies are enforced by the kernel - no cooperation from the realm is required.

### Resource Policy Model

Each realm has a policy specifying minimum and maximum resource allocations:

```
RESOURCE POLICY
════════════════════════════════════════════════════════════════════════

%{:cpu    %{:min <ratio>  :max <ratio>}
  :memory %{:min <bytes>  :max <bytes>}}

Examples:
  %{:cpu %{:max 0.30} :memory %{:max (* 2 +GB+)}}     ; Max 30% CPU, 2GB RAM
  %{:cpu %{:min 0.10 :max 0.50}}                       ; Reserved 10%, max 50%
  %{:memory %{:min (* 512 +MB+) :max (* 4 +GB+)}}     ; Reserved 512MB, max 4GB
```

### Policy Interpretation

| min | max | Meaning |
|-----|-----|---------|
| nil | nil | Best-effort (no guarantees, no limit) |
| nil | 0.30 | Capped at 30%, no reservation |
| 0.10 | nil | Reserved 10%, no upper limit |
| 0.10 | 0.30 | Reserved 10%, capped at 30% |

**min (reservation)**: The realm is guaranteed at least this much resource, even under contention. Other realms cannot starve it.

**max (cap)**: The realm cannot exceed this limit, even if resources are idle. Protects against runaway consumption.

### Hierarchical Budgets

Children share their parent's resource allocation:

```
HIERARCHICAL RESOURCE BUDGETS
════════════════════════════════════════════════════════════════════════

Init Realm (100% of system resources)
├── Drivers Realm: max 30%
│   ├── Network: shares Drivers' 30%
│   ├── UART: shares Drivers' 30%
│   └── Storage: shares Drivers' 30%
│
└── Applications Realm: max 70%
    └── WebServer: shares Applications' 70%
        ├── Worker 1: shares WebServer's budget
        ├── Worker 2: shares WebServer's budget
        └── Worker 3: shares WebServer's budget

Key invariant:
  Network + UART + Storage processes collectively ≤ 30%
  WebServer + all Workers collectively ≤ 70%
```

### Anti-Sybil Protection

Creating child realms cannot increase your resource allocation:

```
ANTI-SYBIL INVARIANT
════════════════════════════════════════════════════════════════════════

Malicious realm has 10% CPU budget.
Tries to create 1000 children to get more CPU:

  for i in range(1000):
      create_child_realm(...)

Result: 1000 children ALL SHARE the parent's 10%
        Total CPU for parent + all children = still 10%
        No amplification possible

This is fundamental to seL4's model - children are carved out of
parent's budget, not added to it.
```

### Mapping Policy to seL4 Mechanisms

| Policy | seL4 Mechanism | Effect |
|--------|----------------|--------|
| **CPU max** | SchedContext budget/period | MCS scheduler enforces CPU limit |
| **CPU min** | SchedContext priority + budget | Higher priority ensures reservation |
| **Memory max** | Untyped capability grants | Cannot retype more than granted |
| **Memory min** | Pre-allocated Untyped caps | Reserved memory always available |

### Example Configurations

```
REALM CONFIGURATION EXAMPLES
════════════════════════════════════════════════════════════════════════

Network Driver:
%{:cpu    %{:min 0.05 :max 0.15}    ; Needs guaranteed CPU for IRQs
  :memory %{:max (* 64 +MB+)}}       ; Limited memory footprint

Database Server:
%{:cpu    %{:max 0.40}               ; Can use up to 40% CPU
  :memory %{:min (* 1 +GB+)          ; Guaranteed 1GB
           :max (* 8 +GB+)}}         ; Can grow to 8GB

Background Worker:
%{:cpu    %{:max 0.10}               ; Low priority, capped
  :memory %{:max (* 256 +MB+)}}      ; Small footprint

Untrusted Plugin:
%{:cpu    %{:max 0.05}               ; Strictly limited
  :memory %{:max (* 32 +MB+)}}       ; Minimal resources
```

### Realm Lifecycle

```
REALM STATES
════════════════════════════════════════════════════════════════════════

            create
   ┌──────────────────┐
   │                  ▼
   │            ┌──────────┐
   │            │  DORMANT │  ← Kernel objects allocated, not running
   │            └────┬─────┘
   │                 │ start
   │                 ▼
   │            ┌──────────┐
   │            │ RUNNING  │  ← Workers executing, processes active
   │            └────┬─────┘
   │                 │ terminate
   │                 ▼
   │            ┌──────────┐
   └────────────│TERMINATED│  ← Resources reclaimed
                └──────────┘
```

**DORMANT**: Kernel objects (VSpace, CSpace, TCBs) exist but workers are suspended. No CPU consumed. Memory for kernel objects is allocated.

**RUNNING**: Workers are active, processes executing. Consumes CPU according to policy.

**TERMINATED**: All resources reclaimed. Untyped memory returned to parent's pool.
