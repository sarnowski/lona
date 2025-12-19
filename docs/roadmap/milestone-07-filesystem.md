## Milestone 7: Filesystem

**Goal**: Implement VFS and FAT filesystem.

**Prerequisite**: Milestone 6 complete

### Phase 7.1: VFS Layer

#### Task 7.1.1: VFS Abstraction

**Description**: Define virtual filesystem abstraction.

**Files to create**:
- `lona/fs/vfs.lona`

**Requirements**:
- `Filesystem` protocol
- `open`, `close`, `read`, `write`
- `stat`, `readdir`
- `mkdir`, `unlink`, `rename`
- Path resolution

**Estimated effort**: 2 context windows

---

#### Task 7.1.2: File Handles

**Description**: Implement file handle management.

**Files to modify**:
- `lona/fs/vfs.lona`

**Requirements**:
- File descriptor table
- Open file tracking
- Position management
- Reference counting

**Estimated effort**: 1-2 context windows

---

#### Task 7.1.3: Path Resolution

**Description**: Implement path parsing and resolution.

**Files to modify**:
- `lona/fs/vfs.lona`

**Requirements**:
- Path parsing (split components)
- Absolute vs relative paths
- Mount point traversal
- `.` and `..` handling

**Estimated effort**: 1 context window

---

### Phase 7.2: FAT Implementation

#### Task 7.2.1: FAT Structures

**Description**: Parse FAT filesystem structures.

**Files to create**:
- `lona/fs/fat.lona`

**Requirements**:
- Boot sector parsing
- FAT table reading
- Directory entry parsing
- Support FAT12, FAT16, FAT32

**Estimated effort**: 2 context windows

---

#### Task 7.2.2: FAT File Operations

**Description**: Implement FAT file operations.

**Files to modify**:
- `lona/fs/fat.lona`

**Requirements**:
- File reading (follow cluster chain)
- File writing (allocate clusters)
- File truncation
- Directory operations

**Estimated effort**: 2-3 context windows

---

#### Task 7.2.3: FAT Directory Operations

**Description**: Implement FAT directory operations.

**Files to modify**:
- `lona/fs/fat.lona`

**Requirements**:
- Directory listing
- File lookup
- Create directory entry
- Delete directory entry
- Long filename support

**Estimated effort**: 2 context windows

---

### Phase 7.3: Integration

#### Task 7.3.1: Filesystem Server

**Description**: Run filesystem as server process.

**Files to create**:
- `lona/fs/server.lona`

**Requirements**:
- GenServer for FS operations
- Concurrent access handling
- Caching layer
- Flush/sync operations

**Estimated effort**: 2 context windows

---

#### Task 7.3.2: Mount System

**Description**: Implement mount/unmount.

**Files to create**:
- `lona/fs/mount.lona`

**Requirements**:
- Mount table management
- Mount filesystem at path
- Unmount with cleanup
- Mount point lookup

**Estimated effort**: 1-2 context windows

---

#### Task 7.3.3: Init Integration

**Description**: Mount root filesystem at boot.

**Files to modify**:
- `lona/init.lona`

**Requirements**:
- Start block driver
- Start filesystem server
- Mount root filesystem
- Read startup manifest

**Estimated effort**: 1 context window

---

#### Task 7.3.4: Filesystem Tests

**Description**: Test filesystem functionality.

**Files to create**:
- `test/fs/fat_test.lona`
- `test/fs/vfs_test.lona`

**Requirements**:
- Read/write tests
- Directory tests
- Path resolution tests
- Concurrent access tests

**Estimated effort**: 2 context windows

---

