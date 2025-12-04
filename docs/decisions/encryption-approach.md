# Encryption Approach Decision

**Date**: 2025-11-24
**Status**: Proposed
**Context**: OAuth Security Remediation (TASK-004)

## Summary

This document outlines the selected encryption approach for securing OAuth tokens and sensitive data in the OpenChime application. The decision focuses on application-layer encryption using battle-tested Rust cryptography crates.

## Problem Statement

OpenChime stores OAuth tokens and refresh tokens in plaintext in SQLite database:
- `accounts.auth_data` column (JSON containing access tokens, refresh tokens, OAuth state)
- `accounts.refresh_token` column

**Threat Model**:
- Malware with filesystem access can steal tokens
- Physical access to device exposes tokens
- Backup files contain plaintext credentials
- Memory dumps may contain unencrypted sensitive data

## Decision

### Encryption Architecture: Application-Layer Encryption

**Selected Approach**: Application-layer encryption (vs. full database encryption like SQLCipher)

**Rationale**:
1. **Portability**: No dependency on SQLCipher builds or licensing concerns
2. **Gradual Migration**: Can encrypt column-by-column without rewriting entire database
3. **Granular Control**: Encrypt only sensitive fields, not entire database (better performance)
4. **Debugging**: Can still inspect non-sensitive data in database during development
5. **Compatibility**: Works with existing `sqlx` crate and SQLite setup

**Trade-offs**:
- ✅ More flexible, easier to audit encryption logic
- ✅ Better compatibility with existing tooling
- ❌ More code to write and maintain vs. transparent encryption
- ❌ Need to manually handle nonce storage and key management

### Encryption Algorithm: AES-256-GCM

**Selected**: AES-256-GCM (Galois/Counter Mode)

**Properties**:
- **Authenticated Encryption**: Provides both confidentiality AND integrity/authenticity
- **Prevents Tampering**: Any modification to ciphertext will be detected during decryption
- **NIST Approved**: FIPS 140-2 compliant algorithm
- **Hardware Acceleration**: AES-NI instructions on modern CPUs (3x+ performance)
- **Key Size**: 256-bit keys (strongest AES variant)
- **Nonce Size**: 96-bit nonces (12 bytes) - must NEVER be reused with same key

**Why Not Alternatives**:
- **AES-CBC**: No authentication - vulnerable to tampering attacks
- **ChaCha20-Poly1305**: Good choice, but AES-GCM has better hardware acceleration on x86/ARM
- **AES-128-GCM**: Weaker key size, no compelling reason to use over AES-256

### Encryption Crate: `aes-gcm` (RustCrypto)

**Selected**: `aes-gcm` version `0.10.3`

**Rationale**:
1. **Pure Rust**: No C dependencies, easier cross-compilation
2. **Security Audit**: Audited by NCC Group (MobileCoin-funded), no significant findings
3. **Hardware Acceleration**: Automatic AES-NI and CLMUL instruction usage on x86/x64
4. **Active Maintenance**: Part of RustCrypto organization (well-maintained ecosystem)
5. **Excellent Documentation**: Clear examples, good API ergonomics
6. **Constant-Time**: Side-channel resistant implementation

**Why Not `ring`**:
- `ring` is excellent (uses BoringSSL kernels), but:
  - More opinionated API, harder to audit
  - Heavier dependency (C code from BoringSSL)
  - Less flexible for our specific use case
  - `aes-gcm` is sufficient for desktop application needs

**Version Selection**:
- **0.10.3** (stable) vs. **0.11.0-rc.2** (release candidate)
- Choose **0.10.3**: Production-ready, well-tested, stable API
- 0.11.0-rc.2 requires Rust 1.85+ (we're on 1.91, but prefer stable crates)

### Key Derivation Function: Argon2id

**Selected**: `argon2` version `0.5.3` (Argon2id variant)

**Rationale**:
1. **Modern Standard**: Winner of Password Hashing Competition (2015), recommended by OWASP
2. **Memory-Hard**: Resistant to GPU/ASIC attacks via configurable memory usage
3. **Argon2id Variant**: Hybrid mode combining Argon2i (side-channel resistant) and Argon2d (GPU-resistant)
4. **Configurable**: Tunable time cost, memory cost, and parallelism
5. **Pure Rust**: Part of RustCrypto ecosystem, well-maintained

**Why Not `pbkdf2`**:
- PBKDF2 is older, less secure against modern hardware attacks
- Not memory-hard: Vulnerable to parallel GPU/ASIC cracking
- PBKDF2 iterations alone don't provide same security as Argon2's memory hardness
- No compelling advantage over Argon2 for new implementations

**Configuration** (recommended for desktop app):
```rust
use argon2::Argon2;

// Argon2id with moderate parameters (balance security vs. latency)
let config = argon2::Config {
    variant: argon2::Variant::Argon2id,
    version: argon2::Version::Version13,
    mem_cost: 65536,      // 64 MiB memory
    time_cost: 3,         // 3 iterations
    lanes: 4,             // 4 parallel threads
};
```

**Version Selection**:
- **0.5.3** (stable) vs. **0.6.0-rc.2** (release candidate)
- Choose **0.5.3**: Production stability over cutting-edge features

### Supporting Dependencies

#### 1. `zeroize` - Secure Memory Wiping

**Version**: `1.8.2` (latest stable)

**Purpose**:
- Securely clear sensitive data (keys, plaintexts) from memory after use
- Prevents memory dumps from leaking secrets
- Compiler fence to prevent optimization from removing zeroing

**Usage**:
```rust
use zeroize::Zeroize;

let mut secret_key = [0u8; 32];
// ... use key ...
secret_key.zeroize(); // Securely wipe from memory
```

**Features Needed**: `["derive"]` for deriving `Zeroize` on custom types

#### 2. `rand` - Cryptographically Secure Random Generation

**Current Version in Project**: Not currently in `Cargo.toml`
**Recommended Version**: `0.8.5` (latest stable 0.8.x series)

**Purpose**:
- Generate cryptographically secure nonces for AES-GCM
- Generate random salt for Argon2 key derivation

**Why Not 0.10.0-rc.5**:
- RC version, stick with stable 0.8.x line
- 0.8.5 is battle-tested and sufficient for our needs

**Features Needed**: `["getrandom"]` for OS-level entropy

#### 3. `base64` - Encoding for Database Storage

**Current Version in Project**: `0.21.x` (already present)
**No Changes Needed**

**Purpose**:
- Encode binary ciphertext + nonce for TEXT column storage in SQLite
- Decode from database on reads

**Alternative Considered**: Store as BLOB type instead of base64-encoded TEXT
- **Decision**: Use base64 + TEXT for easier debugging and compatibility with tools

## Implementation Architecture

### Data Flow

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Master Key Derivation (One-time setup)                   │
│                                                              │
│  Platform Keystore (e.g., macOS Keychain)                  │
│           │                                                  │
│           ├──> Master Secret (256-bit random)               │
│           │                                                  │
│           └──> Argon2id KDF                                 │
│                    │                                         │
│                    └──> Encryption Key (32 bytes)           │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ 2. Encryption (Per-record)                                  │
│                                                              │
│  Plaintext (OAuth token JSON)                               │
│           │                                                  │
│           ├──> Generate Random Nonce (12 bytes)             │
│           │                                                  │
│           └──> AES-256-GCM Encrypt                          │
│                (key, nonce, plaintext)                       │
│                    │                                         │
│                    └──> Ciphertext + Auth Tag (16 bytes)    │
│                              │                               │
│                              └──> Combine: nonce || ciphertext || tag │
│                                       │                      │
│                                       └──> Base64 Encode     │
│                                              │               │
│                                              └──> Store in DB│
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ 3. Decryption (Per-record read)                             │
│                                                              │
│  Base64-encoded ciphertext from DB                          │
│           │                                                  │
│           ├──> Base64 Decode                                │
│           │                                                  │
│           └──> Split: nonce || ciphertext || tag            │
│                    │                                         │
│                    └──> AES-256-GCM Decrypt                 │
│                         (key, nonce, ciphertext, tag)        │
│                              │                               │
│                              ├──> Authentication Check       │
│                              │    (fails if tampered)        │
│                              │                               │
│                              └──> Plaintext (OAuth tokens)   │
└─────────────────────────────────────────────────────────────┘
```

### Database Schema Changes

**Current Schema**:
```sql
CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    email TEXT NOT NULL,
    auth_data TEXT,
    refresh_token TEXT,
    ...
);
```

**Encrypted Schema** (no schema changes needed):
```sql
-- Same columns, but auth_data and refresh_token now store:
-- base64(nonce || ciphertext || auth_tag)
--
-- Format: <12-byte-nonce><variable-ciphertext><16-byte-tag>
-- Encoded as base64 TEXT
```

**Migration Strategy**:
1. Read existing plaintext value
2. Encrypt using new encryption service
3. Write back as base64-encoded ciphertext
4. Mark record as encrypted (or use format detection)

### Module Structure

```
src/
├── crypto/
│   ├── mod.rs              # Public API
│   ├── encryption.rs       # AES-GCM encryption/decryption
│   ├── key_derivation.rs   # Argon2 KDF
│   └── keystore.rs         # Platform keystore integration (future)
├── services/
│   └── account_service.rs  # Modified to use crypto module
└── models/
    └── account.rs          # Account model (unchanged)
```

## Security Considerations

### Strengths

1. **Defense in Depth**: Even if SQLite file is stolen, tokens are encrypted
2. **Authenticated Encryption**: Tampering is detected and rejected
3. **Memory Safety**: Rust's ownership system + `zeroize` prevents leaks
4. **Modern Algorithms**: AES-256-GCM and Argon2id are current best practices
5. **Hardware Acceleration**: AES-NI makes encryption negligible overhead

### Limitations & Risks

1. **Key Management**: Master secret must be stored in platform keystore (out of scope for TASK-004)
   - **Mitigation**: Future task will integrate macOS Keychain / Windows Credential Manager
   - **Temporary**: For initial implementation, may use user-provided password (less secure)

2. **Memory Exposure**: Decrypted tokens exist in memory during use
   - **Mitigation**: Use `zeroize` to clear after use, but can't protect against memory dumps during active use
   - **Accepted Risk**: This is inherent to any application-layer encryption

3. **Nonce Reuse**: Critical security failure if same nonce used twice with same key
   - **Mitigation**: Always generate random nonce per encryption, never reuse
   - **Implementation Note**: Use `rand::thread_rng()` for cryptographic randomness

4. **Key Rotation**: No automatic key rotation mechanism
   - **Accepted Risk**: Low priority for MVP, can add in future
   - **Mitigation**: Document manual re-encryption procedure

5. **Side-Channel Attacks**: Timing attacks possible on key derivation
   - **Mitigation**: Argon2 is designed to be side-channel resistant (constant-time where possible)
   - **Accepted Risk**: Desktop threat model is less severe than server/cloud

## Dependencies Summary

Add to `Cargo.toml`:

```toml
[dependencies]
# Cryptography - OAuth token encryption
aes-gcm = "0.10.3"          # AES-256-GCM authenticated encryption
argon2 = "0.5.3"            # Argon2id key derivation function
rand = "0.8.5"              # Cryptographically secure random generation
zeroize = { version = "1.8", features = ["derive"] }  # Secure memory wiping

# Already present:
# base64 = "0.21"           # For encoding ciphertext in database
```

**Total Additional Compile Time**: ~5-10 seconds (small pure-Rust crates)
**Binary Size Impact**: ~200-300 KB (minimal)
**Runtime Overhead**: < 1ms per encryption/decryption with AES-NI

## Testing Strategy

1. **Unit Tests**: Test encryption/decryption roundtrip with known plaintexts
2. **Nonce Uniqueness**: Verify random nonce generation never repeats (statistical test)
3. **Tamper Detection**: Verify modified ciphertext fails authentication
4. **Key Derivation**: Test Argon2 produces consistent keys from same input
5. **Zeroization**: Test sensitive data is zeroed after use (memory inspection)
6. **Integration**: Test full flow with database read/write

## Future Enhancements

1. **Platform Keystore Integration**: Store master secret in OS keychain (TASK-005)
2. **Key Rotation**: Implement re-encryption with new keys
3. **Backup Encryption**: Extend encryption to database backups
4. **Memory Protection**: Explore `mlock` for preventing swapping of sensitive memory pages
5. **Audit Logging**: Log encryption/decryption events for security monitoring

## References

- **AES-GCM**: [NIST SP 800-38D](https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication800-38d.pdf)
- **Argon2**: [RFC 9106](https://www.rfc-editor.org/rfc/rfc9106.html)
- **RustCrypto**: [https://github.com/RustCrypto](https://github.com/RustCrypto)
- **aes-gcm crate**: [https://docs.rs/aes-gcm/0.10.3](https://docs.rs/aes-gcm/0.10.3)
- **argon2 crate**: [https://docs.rs/argon2/0.5.3](https://docs.rs/argon2/0.5.3)
- **OWASP Password Storage**: [https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)

## Decision Log

- **2025-11-24**: Initial decision document created
- **Reviewers**: Pending review
- **Status**: Awaiting approval before implementation

---

**Next Steps**: Update `Cargo.toml` and verify build (TASK-004)
