## Milestone 11: IP Stack

**Goal**: Implement IPv4 and IPv6.

**Prerequisite**: Milestone 10 complete

### Phase 11.1: IPv4

#### Task 11.1.1: IPv4 Packet Handling

**Description**: Parse and generate IPv4 packets.

**Files to create**:
- `lona/net/ipv4.lona`

**Requirements**:
- IPv4 header parsing
- IPv4 header generation
- Checksum calculation
- Fragment handling (basic)

**Estimated effort**: 1-2 context windows

---

#### Task 11.1.2: IPv4 Routing

**Description**: Implement IPv4 routing.

**Files to modify**:
- `lona/net/ipv4.lona`

**Requirements**:
- Routing table
- Longest prefix match
- Default gateway
- Static route configuration

**Estimated effort**: 1-2 context windows

---

#### Task 11.1.3: IPv4 Interface

**Description**: Manage IPv4 interface addresses.

**Files to modify**:
- `lona/net/ipv4.lona`

**Requirements**:
- Interface address assignment
- Multiple addresses per interface
- Subnet mask handling
- Address validation

**Estimated effort**: 1 context window

---

### Phase 11.2: IPv6

#### Task 11.2.1: IPv6 Packet Handling

**Description**: Parse and generate IPv6 packets.

**Files to create**:
- `lona/net/ipv6.lona`

**Requirements**:
- IPv6 header parsing
- IPv6 header generation
- Extension headers (basic)
- No checksum (relies on upper layers)

**Estimated effort**: 1-2 context windows

---

#### Task 11.2.2: IPv6 Routing

**Description**: Implement IPv6 routing.

**Files to modify**:
- `lona/net/ipv6.lona`

**Requirements**:
- IPv6 routing table
- Longest prefix match
- Link-local addresses
- Static route configuration

**Estimated effort**: 1-2 context windows

---

### Phase 11.3: Integration

#### Task 11.3.1: IP Server

**Description**: Run IP stack as server.

**Files to create**:
- `lona/net/ip.lona`

**Requirements**:
- Unified IP server
- Protocol demultiplexing
- Send/receive interface
- Integration with ARP/NDP

**Estimated effort**: 1-2 context windows

---

#### Task 11.3.2: IP Tests

**Description**: Test IP functionality.

**Files to create**:
- `test/net/ipv4_test.lona`
- `test/net/ipv6_test.lona`

**Requirements**:
- Packet parsing tests
- Routing tests
- Checksum tests

**Estimated effort**: 1-2 context windows

---

