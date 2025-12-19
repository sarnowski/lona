## Milestone 3: UART Driver

**Goal**: Implement abstract UART driver in Lonala with platform-specific implementations.

**Prerequisite**: Milestone 2 complete

### Phase 3.1: UART Abstraction

#### Task 3.1.1: UART Protocol Definition

**Description**: Define UART driver protocol.

**Files to create**:
- `lona/driver/uart.lona`
- `docs/drivers/uart.md`

**Requirements**:
- `UartDriver` protocol
- `read-byte`, `write-byte` operations
- `available?` - data available check
- Configuration (baud, parity, etc.)
- Actor-based interface

**Estimated effort**: 1 context window

---

#### Task 3.1.2: UART GenServer

**Description**: Implement UART as GenServer.

**Files to modify**:
- `lona/driver/uart.lona`

**Requirements**:
- Init with MMIO base address
- Handle `:read`, `:write` calls
- IRQ-based receive notification
- Buffer management

**Estimated effort**: 1-2 context windows

---

### Phase 3.2: Platform Implementations

#### Task 3.2.1: ARM64 UART (PL011)

**Description**: Implement PL011 UART for ARM64.

**Files to create**:
- `lona/driver/uart/pl011.lona`

**Requirements**:
- PL011 register definitions
- Read/write implementation
- IRQ handling
- QEMU virt machine support

**Estimated effort**: 1-2 context windows

---

#### Task 3.2.2: x86_64 UART (16550)

**Description**: Implement 16550 UART for x86_64.

**Files to create**:
- `lona/driver/uart/ns16550.lona`

**Requirements**:
- 16550 register definitions
- Read/write implementation
- IRQ handling
- Port I/O (may need native primitive)

**Estimated effort**: 1-2 context windows

---

### Phase 3.3: Integration

#### Task 3.3.1: UART Driver Domain

**Description**: Run UART driver in isolated domain.

**Files to modify**:
- `lona/driver/uart.lona`

**Requirements**:
- Spawn in own domain
- Grant minimal capabilities
- IPC interface for clients
- Supervision integration

**Estimated effort**: 1-2 context windows

---

#### Task 3.3.2: UART Driver Tests

**Description**: Test UART driver functionality.

**Files to create**:
- `test/driver/uart_test.lona`

**Requirements**:
- Loopback tests
- IRQ notification tests
- Error handling tests

**Estimated effort**: 1 context window

---

