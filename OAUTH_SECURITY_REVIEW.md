# OAuth Token Security Review - OpenChime Calendar Application

**Date:** 2025-11-22  
**Reviewer:** Senior Software Engineer  
**Scope:** Calendar integration modules (Google OAuth2, Proton ICS)

## 🚨 **Critical Security Findings**

### 1. **Plain Text Token Storage in Database** (HIGH RISK)
- **Issue:** OAuth access tokens, refresh tokens, and ICS URLs stored as plain text in SQLite
- **Location:** `src/database/schema.sql:11` - `auth_data TEXT NOT NULL`
- **Impact:** 
  - Database file compromise exposes all authentication credentials
  - No protection against database theft or unauthorized access
  - Tokens usable by attackers with database access

### 2. **Environment Variable Security Gaps** (HIGH RISK)
- **Issue:** OAuth client secrets loaded without validation or security checks
- **Location:** 
  - `src/calendar/google.rs:287` - `std::env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|| "your-client-id".to_string())`
  - `src/calendar/google.rs:289` - `std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_else(|| "your-client-secret".to_string())`
- **Impact:**
  - Fallback to insecure default credentials in production
  - No validation that production credentials are properly set
  - Potential exposure of default/placeholder secrets

### 3. **No Token Encryption or Key Management** (MEDIUM RISK)
- **Issue:** No encryption at rest for sensitive authentication data
- **Impact:**
  - SQLite database provides no built-in encryption
  - No key rotation mechanism
  - No secure key storage

## 🛡️ **Security Recommendations**

### Immediate Actions (HIGH PRIORITY)

#### 1. **Implement Database Encryption**
```rust
// Consider SQLite encryption extensions like SQLCipher
// Or migrate to encrypted database format
```

#### 2. **Validate Environment Variables at Startup**
```rust
// In main.rs or startup routine
fn validate_oauth_credentials() -> Result<()> {
    let client_id = env::var("GOOGLE_CLIENT_ID")
        .map_err(|_| AppError::Config("GOOGLE_CLIENT_ID not set".to_string()))?;
    let client_secret = env::var("GOOGLE_CLIENT_SECRET")
        .map_err(|_| AppError::Config("GOOGLE_CLIENT_SECRET not set".to_string()))?;
    
    // Validate format/structure
    if client_id == "your-client-id" || client_secret == "your-client-secret" {
        return Err(AppError::Config("Default OAuth credentials detected. Set production values.".to_string()));
    }
    
    Ok(())
}
```

#### 3. **Add Token Encryption Layer**
```rust
// Create encryption service for sensitive data
pub struct TokenEncryption {
    key: Vec<u8>, // Should be derived from secure key management
}

impl TokenEncryption {
    pub fn encrypt(&self, token: &str) -> Result<String> {
        // Use AES-GCM or similar for token encryption
        // Store encrypted data in database
    }
    
    pub fn decrypt(&self, encrypted_token: &str) -> Result<String> {
        // Decrypt and return original token
    }
}
```

### Medium-Term Improvements (MEDIUM PRIORITY)

#### 4. **Implement Secure Token Storage Pattern**
```rust
// Store only necessary tokens, encrypt all sensitive data
Account {
    provider: "google",
    account_name: "user@domain.com",
    // Store encrypted OAuth data
    auth_data: encrypt_oauth_tokens(oauth_tokens),
    // Store minimal refresh token separately if needed
    refresh_token: encrypted_refresh_token,
}
```

#### 5. **Add Token Rotation and Expiration Handling**
```rust
// Implement proactive token refresh
async fn refresh_token_if_needed(account: &Account) -> Result<Option<String>> {
    let token_data: GoogleTokenResponse = serde_json::from_str(&account.auth_data)?;
    
    if is_token_expired_soon(&token_data) {
        let new_token = refresh_access_token(account).await?;
        // Update stored token
        return Ok(Some(new_token));
    }
    
    Ok(None)
}
```

#### 6. **Implement Certificate Validation**
```rust
// Ensure HTTPS and certificate validation for all OAuth flows
let client = Client::builder()
    .timeout(Duration::from_secs(30))
    .danger_disable_hostname_verification(false) // Enable proper validation
    .build()?;
```

### Long-Term Security Measures (LOW PRIORITY)

#### 7. **Migrate to Secure Storage Solution**
- Consider platform-specific secure storage (Keychain on macOS, Credential Manager on Windows)
- Implement OS-level encryption for stored credentials
- Use hardware security modules where available

#### 8. **Implement Audit Logging**
```rust
// Log token access and modifications for security auditing
fn log_token_access(account_id: i64, operation: &str, success: bool) {
    // Log to secure audit trail
}
```

## 📋 **Implementation Priority**

| Priority | Task | Effort | Risk Level |
|----------|------|--------|------------|
| P0 | Validate OAuth environment variables | Low | HIGH |
| P0 | Add database encryption layer | Medium | HIGH |
| P1 | Implement token encryption | Medium | MEDIUM |
| P1 | Add token rotation logic | Low | MEDIUM |
| P2 | Migrate to OS secure storage | High | MEDIUM |
| P2 | Implement audit logging | Medium | LOW |

## 🔧 **Files Requiring Changes**

1. **`src/main.rs`** - Add environment validation
2. **`src/database/schema.sql`** - Update for encrypted storage
3. **`src/calendar/google.rs`** - Add encryption, validation
4. **`src/calendar/proton.rs`** - Add URL validation, encryption
5. **New: `src/security/`** - Token encryption service

## ⚠️ **Current Risk Assessment**

- **HIGH RISK:** Database compromise exposes all OAuth credentials
- **HIGH RISK:** Production may run with default OAuth secrets
- **MEDIUM RISK:** No token expiration or rotation security
- **LOW RISK:** Current implementation follows standard OAuth2 patterns

**Recommendation:** Prioritize P0 tasks before production deployment to prevent credential exposure incidents.
