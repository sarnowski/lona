## Milestone 9: Network Driver

**Goal**: Implement VirtIO network driver.

**Prerequisite**: Milestone 5 complete (parallel with M6-M8)

### Phase 9.1: Network Abstraction

#### Task 9.1.1: Network Device Protocol

**Description**: Define network device protocol.

**Files to create**:
- `lona/driver/net.lona`

**Requirements**:
- `NetDevice` protocol
- `send-frame`, `receive-frame`
- MAC address access
- MTU information
- Actor-based interface

**Estimated effort**: 1 context window

---

#### Task 9.1.2: Frame Buffer Management

**Description**: Manage network frame buffers.

**Files to modify**:
- `lona/driver/net.lona`

**Requirements**:
- RX buffer pool
- TX buffer management
- Zero-copy where possible
- Buffer recycling

**Estimated effort**: 1-2 context windows

---

### Phase 9.2: VirtIO Net

#### Task 9.2.1: VirtIO Net Implementation

**Description**: Implement VirtIO net driver.

**Files to create**:
- `lona/driver/net/virtio.lona`

**Requirements**:
- VirtIO net initialization
- RX/TX virtqueue setup
- Frame send/receive
- IRQ handling

**Estimated effort**: 2-3 context windows

---

#### Task 9.2.2: Network Driver Domain

**Description**: Run network driver in isolated domain.

**Files to modify**:
- `lona/driver/net.lona`
- `lona/init/drivers.lona`

**Requirements**:
- Spawn in own domain
- DMA buffer capabilities
- IRQ capabilities
- Client IPC interface

**Estimated effort**: 1 context window

---

### Phase 9.3: Integration

#### Task 9.3.1: Network Driver Tests

**Description**: Test network driver.

**Files to create**:
- `test/driver/net_test.lona`

**Requirements**:
- Frame send/receive tests
- Buffer management tests
- Error handling tests

**Estimated effort**: 1 context window

---

