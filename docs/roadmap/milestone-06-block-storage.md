# Milestone 6: Block Storage

**Goal**: Implement VirtIO block storage driver.

**Prerequisite**: Milestone 5 complete

## Phase 6.1: VirtIO Infrastructure

### Task 6.1.1: VirtQueue Abstraction

**Description**: Implement reusable VirtQueue abstraction for all VirtIO devices.

**Files to create**:
- `lona/driver/virtio/queue.lona`

**Requirements**:
- VirtQueue data structure (descriptor table, available ring, used ring)
- Descriptor chain construction helper
- `alloc-descriptor`, `free-descriptor` - descriptor management
- `add-buffer` - add buffer to available ring
- `get-used` - retrieve completed buffers from used ring
- Memory barrier integration for DMA coherency
- Configurable queue size

**Tests**:
- Queue initialization
- Descriptor allocation/free
- Buffer submission
- Completion retrieval
- Chain construction

**Estimated effort**: 2 context windows

**Note**: This abstraction is shared by block, network, and future VirtIO drivers.

---

### Task 6.1.2: VirtIO Common Layer

**Description**: Implement VirtIO device initialization common to all device types.

**Files to create**:
- `lona/driver/virtio.lona`

**Requirements**:
- VirtIO MMIO register definitions
- Device reset and initialization sequence
- Feature negotiation protocol
- Queue setup coordination
- Device status management

**Estimated effort**: 1-2 context windows

---

### Task 6.1.3: VirtIO Device Discovery

**Description**: Discover VirtIO devices.

**Files to modify**:
- `lona/driver/virtio.lona`

**Requirements**:
- MMIO device discovery
- Device type identification
- Feature negotiation
- Device initialization

**Estimated effort**: 1-2 context windows

---

## Phase 6.2: Block Driver

### Task 6.2.1: Block Device Protocol

**Description**: Define block device protocol.

**Files to create**:
- `lona/driver/block.lona`

**Requirements**:
- `BlockDevice` protocol
- `read-block`, `write-block`
- `block-size`, `block-count`
- Actor-based interface

**Estimated effort**: 1 context window

---

### Task 6.2.2: VirtIO Block Implementation

**Description**: Implement VirtIO block driver.

**Files to create**:
- `lona/driver/block/virtio.lona`

**Requirements**:
- VirtIO block commands
- Async I/O with interrupts
- Request queuing
- Error handling

**Estimated effort**: 2-3 context windows

---

## Phase 6.3: Integration

### Task 6.3.1: Block Driver Domain

**Description**: Run block driver in isolated domain.

**Files to modify**:
- `lona/driver/block.lona`
- `lona/init/drivers.lona`

**Requirements**:
- Spawn in own domain
- DMA buffer capabilities
- IRQ capabilities

**Estimated effort**: 1 context window

---

### Task 6.3.2: Block Driver Tests

**Description**: Test block driver functionality.

**Files to create**:
- `test/driver/block_test.lona`

**Requirements**:
- Read/write tests
- Large I/O tests
- Error handling tests

**Estimated effort**: 1 context window

---

