## Milestone 13: Telnet Server

**Goal**: Implement telnet daemon for remote REPL access.

**Prerequisite**: Milestone 12 complete

### Phase 13.1: Telnet Protocol

#### Task 13.1.1: Telnet Basics

**Description**: Implement telnet protocol.

**Files to create**:
- `lona/net/telnet.lona`

**Requirements**:
- Telnet option negotiation
- Command interpretation
- Line mode handling
- Echo handling

**Estimated effort**: 1-2 context windows

---

### Phase 13.2: Telnet Server

#### Task 13.2.1: Connection Handler

**Description**: Handle telnet connections.

**Files to create**:
- `lona/service/telnetd.lona`

**Requirements**:
- Accept TCP connections
- Spawn per-connection process
- Session management
- Graceful disconnect

**Estimated effort**: 1-2 context windows

---

#### Task 13.2.2: REPL Integration

**Description**: Connect REPL to telnet session.

**Files to modify**:
- `lona/service/telnetd.lona`
- `lona/repl.lona`

**Requirements**:
- REPL reads from socket
- REPL writes to socket
- Per-user domain isolation
- Session cleanup

**Estimated effort**: 1-2 context windows

---

### Phase 13.3: Configuration

#### Task 13.3.1: Boot Configuration

**Description**: Configure REPL sources at boot.

**Files to modify**:
- `lona/init.lona`

**Requirements**:
- Enable/disable UART REPL
- Enable/disable telnet REPL
- Read from boot parameters
- Default configuration

**Estimated effort**: 1 context window

---

#### Task 13.3.2: Telnet Tests

**Description**: Test telnet functionality.

**Files to create**:
- `test/service/telnetd_test.lona`

**Requirements**:
- Connection tests
- Session tests
- Multi-user tests

**Estimated effort**: 1 context window

---

