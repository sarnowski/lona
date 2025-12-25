# Milestone 8: Persistent Storage

**Goal**: Enable applications to persist data.

**Prerequisite**: Milestone 7 complete

## Phase 8.1: Write Operations

### Task 8.1.1: File Write Support

**Description**: Complete file write implementation.

**Files to modify**:
- `lona/fs/fat.lona`
- `lona/fs/server.lona`

**Requirements**:
- Write with cluster allocation
- Append mode
- Atomic write semantics
- Write-through or buffered

**Estimated effort**: 1-2 context windows

---

### Task 8.1.2: Directory Modification

**Description**: Complete directory write support.

**Files to modify**:
- `lona/fs/fat.lona`

**Requirements**:
- Create files/directories
- Delete files/directories
- Rename operations
- Update timestamps

**Estimated effort**: 1-2 context windows

---

## Phase 8.2: Durability

### Task 8.2.1: Sync Operations

**Description**: Ensure durability guarantees.

**Files to modify**:
- `lona/fs/server.lona`
- `lona/driver/block.lona`

**Requirements**:
- `fsync` for file durability
- `sync` for filesystem durability
- Proper ordering of writes

**Estimated effort**: 1 context window

---

### Task 8.2.2: Persistence Tests

**Description**: Test data persistence.

**Files to create**:
- `test/fs/persistence_test.lona`

**Requirements**:
- Write and verify after "reboot"
- Crash recovery tests
- Large file tests

**Estimated effort**: 1 context window

---

