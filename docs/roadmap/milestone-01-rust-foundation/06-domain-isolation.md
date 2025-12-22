## Phase 1.6: Domain Isolation & IPC

Implement seL4-based security domains and inter-domain communication.

---

### Task 1.6.0: Capability Value Type

**Description**: Define the Lonala-level representation of seL4 capabilities.

**Files to modify**:
- `crates/lona-core/src/value/mod.rs`
- `crates/lona-core/src/value/capability.rs` (new)

**Requirements**:
- `Capability` as a distinct Value type (opaque handle to seL4 capability)
- Capability equality (comparing slot references, not contents)
- Capability printing (type and limited info, no secret exposure)
- Capability cannot be forged from integers or other values
- Capability metadata support (type, granted rights, origin)
- Support for capability "badges" (user data attached by granter)

**Design Note**: Capabilities are first-class Lonala values, passed explicitly to primitives that require them. This enables "No Ambient Authority" - all privileged operations require explicit capability arguments.

**Tests**:
- Capability creation (only via system primitives)
- Capability equality
- Capability printing
- Cannot forge from other types
- Metadata access

**Estimated effort**: 1-2 context windows

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

**Description**: Manage seL4 capability spaces with full lifecycle including revocation.

**Files to modify**:
- `crates/lona-runtime/src/domain/cspace.rs` (new)

**Requirements**:
- Create new CSpace
- Allocate capability slots
- Copy/mint capabilities with attenuation
- Delete capabilities
- **Revocation**: `(cap-revoke cap)` invalidates capability and all derivatives
- Revocation tracking (parent→child capability derivation tree)
- Revocation cascades to all descendants per seL4 semantics

**Design Note**: Per goals, "Revocation cascades to all descendants." The derivation tree tracks which capabilities were minted/copied from which, enabling cascade revocation.

**Tests**:
- CSpace creation
- Slot allocation
- Capability copy/mint with attenuation
- Revocation of single capability
- Cascade revocation to derivatives

**Estimated effort**: 2-3 context windows

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
- Message passing across domain boundaries
- Notify target domain
- Transparent to Lonala code (same `send` API)

> ⚠️ **OPEN DESIGN DECISION**: See "Cross-Domain Zero-Copy IPC" in roadmap index. Goals promise zero-copy via immutability, but current description says "serialize/deserialize". Need dedicated design session to resolve: (1) shared-heap for persistent structures, (2) cross-domain GC coordination, (3) hybrid small-copy/large-share approach.

**Tests**:
- Cross-domain send
- Message integrity
- Transparency (same API)

**Estimated effort**: 2-3 context windows (may increase based on design decision)

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
