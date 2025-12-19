## Milestone 12: Transport Protocols

**Goal**: Implement ICMP, UDP, and TCP.

**Prerequisite**: Milestone 11 complete

### Phase 12.1: ICMP

#### Task 12.1.1: ICMPv4 Implementation

**Description**: Implement ICMPv4.

**Files to create**:
- `lona/net/icmp.lona`

**Requirements**:
- Echo request/reply (ping)
- Destination unreachable
- Time exceeded
- Checksum handling

**Estimated effort**: 1-2 context windows

---

#### Task 12.1.2: ICMPv6 Implementation

**Description**: Implement ICMPv6.

**Files to modify**:
- `lona/net/icmp.lona`

**Requirements**:
- Echo request/reply
- Neighbor Discovery basics
- Error messages

**Estimated effort**: 1-2 context windows

---

### Phase 12.2: UDP

#### Task 12.2.1: UDP Protocol

**Description**: Implement UDP.

**Files to create**:
- `lona/net/udp.lona`

**Requirements**:
- UDP header parsing/generation
- Checksum calculation
- Port demultiplexing
- Socket abstraction

**Estimated effort**: 1-2 context windows

---

#### Task 12.2.2: UDP Sockets

**Description**: Implement UDP socket API.

**Files to modify**:
- `lona/net/udp.lona`

**Requirements**:
- `udp-socket` creation
- `bind`, `send`, `recv`
- Broadcast support
- Non-blocking mode

**Estimated effort**: 1-2 context windows

---

### Phase 12.3: TCP

#### Task 12.3.1: TCP State Machine

**Description**: Implement TCP state machine.

**Files to create**:
- `lona/net/tcp.lona`

**Requirements**:
- All TCP states
- State transitions
- Timer management
- Connection tracking

**Estimated effort**: 2-3 context windows

---

#### Task 12.3.2: TCP Connection Setup

**Description**: Implement TCP handshake.

**Files to modify**:
- `lona/net/tcp.lona`

**Requirements**:
- SYN, SYN-ACK, ACK
- Active open (connect)
- Passive open (listen/accept)
- Simultaneous open

**Estimated effort**: 1-2 context windows

---

#### Task 12.3.3: TCP Data Transfer

**Description**: Implement TCP data transfer.

**Files to modify**:
- `lona/net/tcp.lona`

**Requirements**:
- Sequence number handling
- Acknowledgment processing
- Window management
- Data buffering

**Estimated effort**: 2 context windows

---

#### Task 12.3.4: TCP Flow Control

**Description**: Implement TCP flow control.

**Files to modify**:
- `lona/net/tcp.lona`

**Requirements**:
- Sliding window
- Window updates
- Zero window probing
- Silly window syndrome prevention

**Estimated effort**: 1-2 context windows

---

#### Task 12.3.5: TCP Congestion Control

**Description**: Implement basic congestion control.

**Files to modify**:
- `lona/net/tcp.lona`

**Requirements**:
- Slow start
- Congestion avoidance
- Fast retransmit
- Fast recovery

**Estimated effort**: 2 context windows

---

#### Task 12.3.6: TCP Connection Teardown

**Description**: Implement TCP close.

**Files to modify**:
- `lona/net/tcp.lona`

**Requirements**:
- FIN handling
- TIME_WAIT state
- Half-close support
- Reset handling

**Estimated effort**: 1 context window

---

### Phase 12.4: Socket API

#### Task 12.4.1: TCP Sockets

**Description**: Implement TCP socket API.

**Files to create**:
- `lona/net/socket.lona`

**Requirements**:
- `tcp-socket` creation
- `connect`, `listen`, `accept`
- `send`, `recv`
- `close`, `shutdown`

**Estimated effort**: 1-2 context windows

---

### Phase 12.5: Tests

#### Task 12.5.1: Transport Tests

**Description**: Test transport protocols.

**Files to create**:
- `test/net/udp_test.lona`
- `test/net/tcp_test.lona`
- `test/net/icmp_test.lona`

**Requirements**:
- Protocol correctness tests
- Connection lifecycle tests
- Edge case tests

**Estimated effort**: 2-3 context windows

---

