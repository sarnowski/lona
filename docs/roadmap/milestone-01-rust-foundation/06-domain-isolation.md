## Phase 1.6: Domain Isolation & IPC

Implement seL4-based security domains and inter-domain communication.

---

### Task 1.6.1: VSpace Manager

**Description**: Manage seL4 virtual address spaces.

**Files to modify**:
- `crates/lona-runtime/src/domain/vspace.rs` (new)

**Requirements**:
- Create new VSpace (address space)
- Map pages into VSpace
- Unmap pages
- Track mapped regions

**Tests**:
- VSpace creation
- Page mapping
- Region tracking

**Estimated effort**: 2 context windows

---

### Task 1.6.2: CSpace Manager

**Description**: Manage seL4 capability spaces.

**Files to modify**:
- `crates/lona-runtime/src/domain/cspace.rs` (new)

**Requirements**:
- Create new CSpace
- Allocate capability slots
- Copy/mint capabilities
- Delete capabilities

**Tests**:
- CSpace creation
- Slot allocation
- Capability operations

**Estimated effort**: 2 context windows

---

### Task 1.6.3: Domain Data Structure

**Description**: Define domain representation.

**Files to modify**:
- `crates/lona-kernel/src/domain/mod.rs` (new)
- `crates/lona-kernel/src/domain/domain.rs` (new)

**Requirements**:
- Domain contains: VSpace, CSpace, processes, capabilities
- Domain hierarchy (parent/child)
- Domain metadata and naming
- Memory limit tracking

**Tests**:
- Domain creation
- Hierarchy tracking
- Resource limits

**Estimated effort**: 1 context window

---

### Task 1.6.4: Domain Creation Primitive

**Description**: Implement domain spawning.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-runtime/src/domain/mod.rs`

**Requirements**:
- `spawn` with `:domain` option creates new domain
- Capability list specifies granted caps
- Memory limit specification
- Metadata attachment

**Tests**:
- Domain spawn
- Capability granting
- Memory limits
- Metadata

**Estimated effort**: 2 context windows

---

### Task 1.6.5: Shared Memory Regions

**Description**: Implement shared memory for IPC.

**Files to modify**:
- `crates/lona-runtime/src/domain/shared.rs` (new)

**Requirements**:
- Allocate physically contiguous region
- Map into multiple domains
- Capability-controlled access
- Ring buffer structure for messages

**Tests**:
- Region allocation
- Multi-domain mapping
- Access control

**Estimated effort**: 2 context windows

---

### Task 1.6.6: Inter-Domain IPC - Notification

**Description**: Use seL4 notifications for IPC signaling.

**Files to modify**:
- `crates/lona-runtime/src/domain/ipc.rs` (new)

**Requirements**:
- Create notification endpoints
- Signal on message send
- Wait for notification
- Integrate with scheduler

**Tests**:
- Notification send/receive
- Integration with wait

**Estimated effort**: 1-2 context windows

---

### Task 1.6.7: Inter-Domain IPC - Message Passing

**Description**: Implement cross-domain send/receive.

**Files to modify**:
- `crates/lona-kernel/src/process/mod.rs`
- `crates/lona-runtime/src/domain/ipc.rs`

**Requirements**:
- Serialize message to shared buffer
- Notify target domain
- Deserialize on receive
- Transparent to Lonala code

**Tests**:
- Cross-domain send
- Message integrity
- Transparency (same API)

**Estimated effort**: 2 context windows

---

### Task 1.6.8: Capability Transfer

**Description**: Transfer capabilities across domains.

**Files to modify**:
- `crates/lona-runtime/src/domain/ipc.rs`
- `crates/lona-runtime/src/domain/cspace.rs`

**Requirements**:
- Send capability with message
- Receive and install capability
- Attenuation during transfer
- Revocation support

**Tests**:
- Capability send
- Attenuation
- Revocation cascades

**Estimated effort**: 2 context windows

---

### Task 1.6.9: Code Sharing Between Domains

**Description**: Share bytecode read-only across domains.

**Files to modify**:
- `crates/lona-runtime/src/domain/code.rs` (new)

**Requirements**:
- Map bytecode pages read-only to children
- Clone dispatch table to children
- Isolation of hot patches

**Tests**:
- Code sharing
- Dispatch table isolation
- Hot patch isolation

**Estimated effort**: 1-2 context windows
