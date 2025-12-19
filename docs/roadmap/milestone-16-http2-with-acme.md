## Milestone 16: HTTP/2 with ACME

**Goal**: Implement HTTP/2 server with automatic certificate management.

**Prerequisite**: Milestone 14, Milestone 15 complete

### Phase 16.1: HTTP/2 Protocol

#### Task 16.1.1: HTTP/2 Framing

**Description**: Implement HTTP/2 frame layer.

**Files to create**:
- `lona/net/http2.lona`

**Requirements**:
- Frame parsing
- Frame generation
- Frame types (DATA, HEADERS, etc.)
- Connection preface

**Estimated effort**: 2 context windows

---

#### Task 16.1.2: HTTP/2 Streams

**Description**: Implement HTTP/2 streams.

**Files to modify**:
- `lona/net/http2.lona`

**Requirements**:
- Stream multiplexing
- Stream states
- Priority handling
- Flow control

**Estimated effort**: 2-3 context windows

---

#### Task 16.1.3: HPACK

**Description**: Implement HPACK header compression.

**Files to create**:
- `lona/net/hpack.lona`

**Requirements**:
- Static table
- Dynamic table
- Huffman coding
- Header encoding/decoding

**Estimated effort**: 2-3 context windows

---

### Phase 16.2: HTTP/2 Server

#### Task 16.2.1: HTTP/2 Server Core

**Description**: Implement HTTP/2 server.

**Files to create**:
- `lona/service/http2d.lona`

**Requirements**:
- Connection handling
- Request processing
- Response generation
- Server push (optional)

**Estimated effort**: 2-3 context windows

---

#### Task 16.2.2: HTTP/1-HTTP/2 Upgrade

**Description**: Support protocol upgrade.

**Files to modify**:
- `lona/service/httpd.lona`
- `lona/service/http2d.lona`

**Requirements**:
- ALPN negotiation
- HTTP/1.1 upgrade header
- Unified server interface

**Estimated effort**: 1-2 context windows

---

### Phase 16.3: ACME

#### Task 16.3.1: ACME Client

**Description**: Implement ACME protocol client.

**Files to create**:
- `lona/service/acme.lona`

**Requirements**:
- Account creation
- Order creation
- Challenge handling (HTTP-01)
- Certificate retrieval

**Estimated effort**: 2-3 context windows

---

#### Task 16.3.2: Certificate Renewal

**Description**: Automatic certificate renewal.

**Files to modify**:
- `lona/service/acme.lona`

**Requirements**:
- Expiration monitoring
- Automatic renewal
- Certificate installation
- Failure handling

**Estimated effort**: 1-2 context windows

---

### Phase 16.4: Tests

#### Task 16.4.1: HTTP/2 Tests

**Description**: Test HTTP/2 functionality.

**Files to create**:
- `test/net/http2_test.lona`
- `test/net/hpack_test.lona`

**Requirements**:
- Frame tests
- Stream tests
- Flow control tests

**Estimated effort**: 2 context windows

---

#### Task 16.4.2: ACME Tests

**Description**: Test ACME functionality.

**Files to create**:
- `test/service/acme_test.lona`

**Requirements**:
- Challenge tests
- Certificate tests
- Renewal tests

**Estimated effort**: 1 context window

---

