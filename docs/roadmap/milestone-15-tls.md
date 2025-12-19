## Milestone 15: TLS

**Goal**: Implement TLS 1.2/1.3 for secure connections.

**Prerequisite**: Milestone 12 complete

### Phase 15.1: Cryptographic Primitives

#### Task 15.1.1: Hash Functions

**Description**: Implement cryptographic hashes.

**Files to create**:
- `lona/crypto/hash.lona`

**Requirements**:
- SHA-256
- SHA-384
- HMAC

**Estimated effort**: 2-3 context windows

---

#### Task 15.1.2: Symmetric Encryption

**Description**: Implement symmetric ciphers.

**Files to create**:
- `lona/crypto/cipher.lona`

**Requirements**:
- AES-128-GCM
- AES-256-GCM
- ChaCha20-Poly1305

**Estimated effort**: 3-4 context windows

---

#### Task 15.1.3: Asymmetric Cryptography

**Description**: Implement public-key crypto.

**Files to create**:
- `lona/crypto/pubkey.lona`

**Requirements**:
- RSA (basic operations)
- ECDSA with P-256
- X25519 key exchange

**Estimated effort**: 4-5 context windows

---

### Phase 15.2: TLS Protocol

#### Task 15.2.1: TLS Record Layer

**Description**: Implement TLS record protocol.

**Files to create**:
- `lona/net/tls.lona`

**Requirements**:
- Record parsing
- Record generation
- Encryption/decryption
- MAC handling

**Estimated effort**: 2 context windows

---

#### Task 15.2.2: TLS Handshake - Client

**Description**: Implement TLS client handshake.

**Files to modify**:
- `lona/net/tls.lona`

**Requirements**:
- ClientHello
- ServerHello processing
- Certificate validation
- Key exchange

**Estimated effort**: 2-3 context windows

---

#### Task 15.2.3: TLS Handshake - Server

**Description**: Implement TLS server handshake.

**Files to modify**:
- `lona/net/tls.lona`

**Requirements**:
- ServerHello
- Certificate sending
- Key exchange
- Finished verification

**Estimated effort**: 2-3 context windows

---

#### Task 15.2.4: TLS 1.3 Support

**Description**: Add TLS 1.3 specifics.

**Files to modify**:
- `lona/net/tls.lona`

**Requirements**:
- 1-RTT handshake
- 0-RTT (optional)
- New cipher suites
- Key derivation

**Estimated effort**: 2-3 context windows

---

### Phase 15.3: Certificate Management

#### Task 15.3.1: X.509 Parsing

**Description**: Parse X.509 certificates.

**Files to create**:
- `lona/crypto/x509.lona`

**Requirements**:
- Certificate parsing
- Chain validation
- Common name extraction
- Expiration checking

**Estimated effort**: 2-3 context windows

---

#### Task 15.3.2: Certificate Storage

**Description**: Store and manage certificates.

**Files to modify**:
- `lona/crypto/x509.lona`

**Requirements**:
- Certificate file loading
- Private key loading
- Certificate chain building
- Trust store

**Estimated effort**: 1-2 context windows

---

### Phase 15.4: Tests

#### Task 15.4.1: Crypto Tests

**Description**: Test cryptographic primitives.

**Files to create**:
- `test/crypto/hash_test.lona`
- `test/crypto/cipher_test.lona`
- `test/crypto/pubkey_test.lona`

**Requirements**:
- Test vectors from standards
- Edge case tests

**Estimated effort**: 2 context windows

---

#### Task 15.4.2: TLS Tests

**Description**: Test TLS implementation.

**Files to create**:
- `test/net/tls_test.lona`

**Requirements**:
- Handshake tests
- Data transfer tests
- Error handling tests

**Estimated effort**: 2 context windows

---

