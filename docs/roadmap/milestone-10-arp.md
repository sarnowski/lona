## Milestone 10: ARP

**Goal**: Implement Address Resolution Protocol.

**Prerequisite**: Milestone 9 complete

### Phase 10.1: ARP Implementation

#### Task 10.1.1: ARP Table

**Description**: Implement ARP cache.

**Files to create**:
- `lona/net/arp.lona`

**Requirements**:
- IP to MAC mapping
- Entry expiration
- Static entry support
- Cache size limits

**Estimated effort**: 1 context window

---

#### Task 10.1.2: ARP Protocol

**Description**: Implement ARP request/reply.

**Files to modify**:
- `lona/net/arp.lona`

**Requirements**:
- ARP request generation
- ARP reply handling
- ARP announcement
- Gratuitous ARP

**Estimated effort**: 1-2 context windows

---

#### Task 10.1.3: ARP Server

**Description**: Run ARP as server process.

**Files to modify**:
- `lona/net/arp.lona`

**Requirements**:
- GenServer for ARP
- Resolve IP to MAC
- Handle incoming ARP
- Timeout and retry

**Estimated effort**: 1-2 context windows

---

#### Task 10.1.4: ARP Tests

**Description**: Test ARP functionality.

**Files to create**:
- `test/net/arp_test.lona`

**Requirements**:
- Request/reply tests
- Cache behavior tests
- Timeout tests

**Estimated effort**: 1 context window

---

