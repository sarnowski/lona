## Milestone 14: HTTP/1 Server

**Goal**: Implement HTTP/1.1 server for static files.

**Prerequisite**: Milestone 12, Milestone 7 complete

### Phase 14.1: HTTP Protocol

#### Task 14.1.1: HTTP Request Parsing

**Description**: Parse HTTP/1.1 requests.

**Files to create**:
- `lona/net/http.lona`

**Requirements**:
- Request line parsing
- Header parsing
- Body handling
- Chunked transfer decoding

**Estimated effort**: 1-2 context windows

---

#### Task 14.1.2: HTTP Response Generation

**Description**: Generate HTTP/1.1 responses.

**Files to modify**:
- `lona/net/http.lona`

**Requirements**:
- Status line generation
- Header generation
- Body transmission
- Chunked transfer encoding

**Estimated effort**: 1-2 context windows

---

### Phase 14.2: HTTP Server

#### Task 14.2.1: HTTP Server Core

**Description**: Implement HTTP server.

**Files to create**:
- `lona/service/httpd.lona`

**Requirements**:
- Accept connections
- Request routing
- Response sending
- Keep-alive support

**Estimated effort**: 2 context windows

---

#### Task 14.2.2: Static File Serving

**Description**: Serve files from filesystem.

**Files to modify**:
- `lona/service/httpd.lona`

**Requirements**:
- Path to file mapping
- Content-Type detection
- Range requests
- Directory listing

**Estimated effort**: 1-2 context windows

---

#### Task 14.2.3: Error Handling

**Description**: HTTP error responses.

**Files to modify**:
- `lona/service/httpd.lona`

**Requirements**:
- 404 Not Found
- 500 Internal Error
- Custom error pages
- Logging

**Estimated effort**: 1 context window

---

### Phase 14.3: Integration

#### Task 14.3.1: HTTP Configuration

**Description**: Configure HTTP server.

**Files to modify**:
- `lona/service/httpd.lona`
- `lona/init.lona`

**Requirements**:
- Port configuration
- Document root
- Virtual hosts (basic)
- Startup integration

**Estimated effort**: 1 context window

---

#### Task 14.3.2: HTTP Tests

**Description**: Test HTTP server.

**Files to create**:
- `test/service/httpd_test.lona`

**Requirements**:
- Request/response tests
- Static file tests
- Keep-alive tests
- Error handling tests

**Estimated effort**: 1-2 context windows

---

