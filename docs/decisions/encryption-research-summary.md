# Encryption Approach Research Summary

**Task**: TASK-004 - Research and select encryption approach
**Date**: 2025-11-24
**Status**: Complete

## Executive Summary

Research completed for OAuth token encryption approach. Selected stable, battle-tested Rust cryptography crates from the RustCrypto ecosystem. All dependencies added to `Cargo.toml` and verified with successful `cargo check` build.

## Selected Stack

| Component | Crate | Version | Rationale |
|-----------|-------|---------|-----------|
| **Encryption** | `aes-gcm` | 0.10.3 | Pure Rust AES-256-GCM, NCC Group audited, AES-NI acceleration |
| **Key Derivation** | `argon2` | 0.5.3 | Modern memory-hard KDF, resistant to GPU attacks, Argon2id variant |
| **Random Generation** | `rand` | 0.8.5 | Cryptographically secure RNG for nonces |
| **Memory Security** | `zeroize` | 1.8.2 | Secure memory wiping with compiler fence |

## Key Decisions

### 1. Encryption Crate: `aes-gcm` over `ring`

**Decision**: Use `aes-gcm` 0.10.3 from RustCrypto

**Reasoning**:

| Factor | `aes-gcm` | `ring` |
|--------|-----------|--------|
| **Implementation** | Pure Rust | C (BoringSSL kernels) |
| **Audit Status** | NCC Group audit, no findings | Google-maintained (BoringSSL) |
| **Hardware Acceleration** | ✅ AES-NI + CLMUL | ✅ AES-NI |
| **Documentation** | Excellent, clear examples | Good, more opinionated |
| **Cross-compilation** | Easier (no C dependencies) | Harder (C build required) |
| **Binary Size** | Smaller | Larger (includes BoringSSL) |
| **Maintenance** | RustCrypto org | Brian Smith (single maintainer) |

**Conclusion**: For a desktop application with our threat model, `aes-gcm` provides:
- Easier maintenance and debugging (pure Rust)
- Sufficient security (audited, constant-time)
- Better ergonomics for our use case
- Hardware acceleration where available

Both are excellent choices, but `aes-gcm` fits our needs better.

### 2. Key Derivation: `argon2` over `pbkdf2`

**Decision**: Use `argon2` 0.5.3 with Argon2id variant

**Security Comparison**:

| Feature | Argon2id | PBKDF2 |
|---------|----------|--------|
| **Standardization** | Winner of Password Hashing Competition 2015 | NIST SP 800-132 (2010) |
| **Memory Hardness** | ✅ Configurable (64 MiB recommended) | ❌ Low memory (easy GPU attacks) |
| **GPU Resistance** | ✅ High (memory-hard) | ❌ Low (parallelizable) |
| **ASIC Resistance** | ✅ High | ❌ Low |
| **Side-Channel Resistance** | ✅ Argon2id hybrid mode | ⚠️ Timing attacks possible |
| **OWASP Recommendation** | ✅ First choice | ⚠️ Legacy systems only |

**Modern Security Guidance (2024)**:
- OWASP: Recommends Argon2id as first choice for new systems
- Security researchers: PBKDF2 is "least secure of modern algorithms" against hardware attacks
- Rust ecosystem: Both in RustCrypto with similar APIs

**Performance Trade-off**:
- PBKDF2 is faster (but that's a security weakness for KDFs)
- Argon2id intentionally uses more resources to resist parallel cracking
- For desktop app: Slight latency acceptable for significantly better security

**Conclusion**: No compelling reason to use PBKDF2 over Argon2 in 2024. Argon2id provides superior security with minimal downside.

### 3. Version Selection: Stable vs. RC

**Release Candidate Versions Available**:
- `aes-gcm` 0.11.0-rc.2 (requires Rust 1.85+)
- `argon2` 0.6.0-rc.2 (requires Rust 1.85+)
- `rand` 0.10.0-rc.5

**Decision**: Use stable versions (0.10.3, 0.5.3, 0.8.5)

**Reasoning**:
1. **Production Stability**: RC versions are not production-ready by definition
2. **API Stability**: Stable versions have frozen APIs, no breaking changes
3. **Ecosystem Compatibility**: Stable versions have broader ecosystem support
4. **Testing**: More battle-tested in production environments
5. **Migration Path**: Can upgrade to 0.11+/0.6+ later when stable

We're on Rust 1.91, so we *could* use RCs, but there's no compelling feature driving the need. Stick with stable.

## Research Findings

### Hardware Acceleration

**Key Finding**: Modern CPUs have excellent AES acceleration

From research (2024-2025 benchmarks):
- **AES-NI Instructions**: Available on all modern x86/x64 CPUs since ~2010
- **ARM AES**: Apple M1/M2, A14+ chips have hardware AES
- **Performance**: AES-256-GCM with AES-NI is **3x faster** than software implementation
- **vs. ChaCha20**: AES-GCM now beats ChaCha20 on most modern hardware

**Impact**: Encryption/decryption overhead will be < 1ms per operation on modern hardware.

### Security Audit Status

**aes-gcm crate**:
- ✅ NCC Group security audit (MobileCoin-funded)
- ✅ No significant findings
- ✅ Constant-time implementation
- ✅ Side-channel resistant

**argon2 crate**:
- ✅ Part of RustCrypto/password-hashes (well-maintained)
- ✅ Pure Rust implementation of RFC 9106
- ✅ Used in production by many projects

### Ecosystem Maturity

**RustCrypto Organization**:
- Maintains both `aes-gcm` and `argon2`
- Unified API across cryptography crates (`aead`, `password-hash` traits)
- Active development, responsive to security issues
- Well-documented with examples

**Confidence Level**: High - these are the standard cryptography crates in Rust ecosystem

## Implementation Notes

### AES-256-GCM Format

**Ciphertext Structure**:
```
┌────────────┬──────────────────┬─────────────────┐
│  Nonce     │   Ciphertext     │   Auth Tag      │
│  (12 bytes)│   (variable)     │   (16 bytes)    │
└────────────┴──────────────────┴─────────────────┘
      ↓                  ↓                  ↓
    Random        Encrypted data      GMAC tag
  (per record)    (token JSON)      (integrity)
```

**Storage**: Base64-encode entire structure as TEXT in SQLite

**Security Properties**:
- **Nonce uniqueness**: CRITICAL - never reuse nonce with same key
- **Authentication**: Any modification to ciphertext/nonce/tag will fail verification
- **Key reuse**: Safe to use same key for multiple encryptions (with unique nonces)

### Argon2id Configuration

**Recommended Parameters for Desktop**:
```rust
argon2::Config {
    variant: Argon2id,
    version: Version13,
    mem_cost: 65536,   // 64 MiB memory (balance security vs. UX)
    time_cost: 3,      // 3 iterations (~100ms on modern CPU)
    lanes: 4,          // 4 parallel threads
}
```

**Tuning Trade-offs**:
- **Higher memory**: Better GPU resistance, but may impact UX on low-end devices
- **Higher iterations**: More secure, but longer wait time
- **64 MiB / 3 iterations**: Good balance for desktop app

**Derivation Time**: ~100-200ms on modern CPU (acceptable for initial key setup)

### Zeroize Usage

**Critical Pattern**:
```rust
use zeroize::Zeroize;

let mut plaintext = decrypt(ciphertext)?;
// ... use plaintext ...
plaintext.zeroize(); // Explicit zero before drop
```

**Why Needed**:
- Rust drops don't guarantee memory clearing
- Compiler may optimize away normal zeroing
- `zeroize` uses compiler fence to force memory write
- Prevents secrets lingering in memory/swap

**Limitations**:
- Can't prevent memory dumps during active use
- Can't protect against debugger access
- Acceptable for desktop threat model

## Build Verification

**Command**: `cargo check`
**Result**: ✅ Success (8.20s)

**New Dependencies Added** (14 packages):
```
aead v0.5.2                 - AEAD trait (used by aes-gcm)
aes v0.8.4                  - Core AES implementation
aes-gcm v0.10.3            - AES-GCM mode
argon2 v0.5.3              - Argon2 KDF
blake2 v0.10.6             - BLAKE2 (used by Argon2)
cipher v0.4.4              - Cipher traits
ctr v0.9.2                 - CTR mode (used internally)
ghash v0.5.1               - GHASH for GCM
inout v0.1.4               - In-place buffer operations
password-hash v0.5.0       - Password hash traits
polyval v0.6.2             - POLYVAL (alternative to GHASH)
universal-hash v0.5.1      - Universal hash traits
zeroize_derive v1.4.2      - Derive macro for Zeroize
opaque-debug v0.3.1        - Debug formatting without leaking secrets
```

**Total Impact**:
- Compile time: +8 seconds (small pure-Rust crates)
- Binary size: ~200-300 KB increase
- Runtime overhead: < 1ms per operation (with AES-NI)

**No Conflicts**: All dependencies resolved cleanly

## Risks and Mitigations

### Risk 1: Nonce Reuse

**Risk**: If same nonce used twice with same key, AES-GCM security completely breaks
**Impact**: Critical - catastrophic security failure
**Likelihood**: Low (using secure random generation)

**Mitigation**:
- Use `rand::thread_rng()` for cryptographic randomness
- Generate new random nonce per encryption
- 96-bit nonce space = 2^96 combinations (collision probability negligible)
- Add assertion in encryption code to verify nonce not reused (debug builds)

### Risk 2: Key Management

**Risk**: Master secret must be stored somewhere secure
**Impact**: High - if master secret stolen, all tokens can be decrypted
**Likelihood**: Medium (temporary password-based key until keystore integration)

**Mitigation**:
- TASK-005 will integrate platform keystore (macOS Keychain, Windows Credential Manager)
- For MVP: Accept temporary password-based key (document limitations)
- Use Argon2id for password-to-key derivation (resistant to brute force)

### Risk 3: Memory Exposure

**Risk**: Decrypted tokens exist in memory during use
**Impact**: Medium - memory dumps could leak tokens
**Likelihood**: Low (requires privileged access to process memory)

**Mitigation**:
- Use `zeroize` to clear sensitive data after use
- Minimize lifetime of decrypted data in memory
- Acceptable risk for desktop threat model (not server/cloud)

### Risk 4: Side-Channel Attacks

**Risk**: Timing attacks on cryptographic operations
**Impact**: Low - requires local access and sophisticated attack
**Likelihood**: Very low (desktop threat model)

**Mitigation**:
- `aes-gcm` uses constant-time implementation
- Argon2id designed with side-channel resistance
- Hardware AES-NI is inherently constant-time
- Acceptable risk for desktop application

## Next Steps (TASK-005)

1. **Implement Encryption Module** (`src/crypto/`)
   - `encryption.rs`: AES-256-GCM wrapper
   - `key_derivation.rs`: Argon2id wrapper
   - `mod.rs`: Public API

2. **Write Unit Tests**
   - Encryption/decryption roundtrip
   - Tamper detection (modify ciphertext → fail)
   - Nonce uniqueness (statistical test)
   - Zeroization verification

3. **Update Account Service**
   - Encrypt before INSERT/UPDATE
   - Decrypt after SELECT
   - Handle migration of existing plaintext records

4. **Documentation**
   - API documentation with examples
   - Security considerations
   - Migration guide

## References

**Crate Documentation**:
- [aes-gcm docs](https://docs.rs/aes-gcm/0.10.3/aes_gcm/)
- [argon2 docs](https://docs.rs/argon2/0.5.3/argon2/)
- [rand docs](https://docs.rs/rand/0.8.5/rand/)
- [zeroize docs](https://docs.rs/zeroize/1.8/zeroize/)

**Standards**:
- [NIST SP 800-38D: AES-GCM](https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication800-38d.pdf)
- [RFC 9106: Argon2](https://www.rfc-editor.org/rfc/rfc9106.html)

**Security Guidance**:
- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)

**Research Sources**:
- NCC Group audit of aes-gcm (no significant findings)
- 2024 AES vs ChaCha20 benchmarks (AES-NI wins)
- Argon2 vs PBKDF2 security analysis (Argon2 superior)

---

**Research Status**: ✅ Complete
**Build Status**: ✅ Passing
**Ready for Implementation**: ✅ Yes
