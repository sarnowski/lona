## Milestone 4: Init System

**Goal**: Implement Lonala init system that bootstraps the OS.

**Prerequisite**: Milestone 3 complete

### Phase 4.1: Init Process

#### Task 4.1.1: Init Main Function

**Description**: Create init process entry point.

**Files to create**:
- `lona/init.lona`

**Requirements**:
- `lona.init/main` entry point
- Platform detection
- Driver initialization sequence
- Supervision tree root

**Estimated effort**: 1-2 context windows

---

#### Task 4.1.2: Platform Detection

**Description**: Detect hardware platform.

**Files to modify**:
- `lona/init.lona`

**Requirements**:
- Detect ARM64 vs x86_64
- Parse device tree if available
- Select appropriate drivers

**Estimated effort**: 1 context window

---

### Phase 4.2: Driver Supervision

#### Task 4.2.1: Driver Supervisor

**Description**: Supervise device drivers.

**Files to create**:
- `lona/init/drivers.lona`

**Requirements**:
- Supervisor for driver processes
- Restart strategy (one-for-one)
- Driver dependency ordering
- Health monitoring

**Estimated effort**: 1-2 context windows

---

#### Task 4.2.2: UART Initialization

**Description**: Start UART driver from init.

**Files to modify**:
- `lona/init.lona`
- `lona/init/drivers.lona`

**Requirements**:
- Start appropriate UART driver
- Grant UART capabilities
- Register as system console

**Estimated effort**: 1 context window

---

### Phase 4.3: Rust Handoff

#### Task 4.3.1: Boot Handoff

**Description**: Modify Rust runtime to start Lonala init.

**Files to modify**:
- `crates/lona-runtime/src/main.rs`

**Requirements**:
- Load embedded Lonala code
- Start init process
- Hand over console to Lonala
- Keep minimal Rust logging for panics

**Estimated effort**: 1 context window

---

