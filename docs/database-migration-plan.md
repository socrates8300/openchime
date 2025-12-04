# Database Migration Plan: OAuth Token Encryption

**Document Version**: 1.0
**Created**: 2025-11-24
**Status**: Design Phase

## Executive Summary

This document outlines the strategy for migrating existing OpenChime databases from plaintext OAuth token storage to encrypted storage. The migration must be automatic, safe, and reversible.

**Key Requirements**:
- ✅ Automatic migration on application startup
- ✅ Zero user intervention required
- ✅ Automatic backup before migration
- ✅ Idempotent (safe to run multiple times)
- ✅ Rollback capability on failure
- ✅ Support for partial migration states

---

## Migration Approach

### Online vs Offline Migration

**Decision: Online Migration (Automatic on Startup)**

**Rationale**:
- **Better UX**: No manual scripts, no user intervention
- **Safer**: Application not running during critical database operations
- **Validated**: Migration success verified immediately on startup
- **Simpler Recovery**: Automatic rollback on failure

**Rejected Alternative: Offline Migration**
- Requires users to run manual scripts
- Higher risk of user error
- Poor UX for non-technical users
- Difficult to validate before app launch

### Migration Timing

**When**: First application startup after update to encryption-enabled version

**Trigger**: Migration runs automatically in `Database::new()` after schema is initialized

**Duration**: Expected <5 seconds for typical user with <50 accounts

---

## Schema Versioning System

### Design: Simple Version Tracking Table

We'll use a dedicated `schema_migrations` table to track which migrations have been applied.

**Table Structure**:
```sql
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    checksum TEXT -- SHA-256 hash of migration content (future-proofing)
);
```

**Versioning Scheme**:
- **Version 1**: Baseline schema (existing state)
- **Version 2**: Add encryption metadata columns
- **Version 3**: Encrypt existing plaintext tokens (one-time data migration)
- **Version N**: Future migrations

**Why This Approach**:
- **Simple**: No complex migration framework needed (YAGNI)
- **Explicit**: Clear tracking of what's been applied
- **Idempotent**: Check `schema_migrations` before running each migration
- **Debuggable**: Can see exactly which migrations have run
- **Future-Proof**: Supports additional migrations (key rotation, schema changes)

**Integration with Existing Code**:
- Current `ensure_migrations()` in `src/database/mod.rs` uses PRAGMA table_info checks
- New approach: Check `schema_migrations` table for version numbers
- Backward compatible: Can coexist with existing PRAGMA checks

---

## Migration SQL Design

### Migration 001: Baseline (Mark Existing Schema)

**Purpose**: Mark existing databases as "version 1" (no changes, just tracking)

**SQL**:
```sql
-- Create migrations tracking table
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    checksum TEXT
);

-- Mark baseline schema as version 1
INSERT OR IGNORE INTO schema_migrations (version, name)
VALUES (1, 'baseline_schema');
```

**When**: Run on all databases (new and existing)
**Idempotency**: `INSERT OR IGNORE` prevents duplicates

---

### Migration 002: Add Encryption Metadata Columns

**Purpose**: Add columns to track encryption status for each account

**SQL**:
```sql
-- Add encryption_version column (tracks which encryption algorithm used)
-- NULL or 0 = plaintext (not encrypted)
-- 1 = AES-256-GCM with platform keystore
-- 2+ = Future encryption versions (e.g., post-quantum)
ALTER TABLE accounts ADD COLUMN encryption_version INTEGER DEFAULT NULL;

-- Add encrypted_at timestamp (when tokens were last encrypted)
ALTER TABLE accounts ADD COLUMN encrypted_at DATETIME DEFAULT NULL;

-- Mark migration as complete
INSERT OR IGNORE INTO schema_migrations (version, name)
VALUES (2, 'add_encryption_metadata');
```

**When**: Run if `schema_migrations.version = 2` not present
**Impact**: Additive only, no data changes, backward compatible
**Rollback**: Can drop columns if needed (though not recommended)

---

### Migration 003: Encrypt Existing Plaintext Tokens (DATA MIGRATION)

**Purpose**: Re-encrypt all existing accounts that have plaintext tokens

**This is NOT a SQL file** - it's a Rust function that:
1. Queries all accounts with `encryption_version IS NULL OR encryption_version = 0`
2. For each account:
   - Decrypt/read plaintext auth_data and refresh_token (currently stored as plaintext)
   - Encrypt using Account::encrypt_auth_data() and encrypt_refresh_token()
   - Update database with encrypted values
   - Set encryption_version = 1, encrypted_at = NOW()
3. Mark migration complete in schema_migrations

**Pseudo-code**:
```rust
async fn migrate_encrypt_tokens(pool: &SqlitePool) -> Result<()> {
    // Check if already migrated
    let migrated = sqlx::query("SELECT 1 FROM schema_migrations WHERE version = 3")
        .fetch_optional(pool)
        .await?
        .is_some();

    if migrated {
        info!("Migration 003 already applied, skipping");
        return Ok(());
    }

    // Find accounts with plaintext tokens
    let plaintext_accounts = sqlx::query_as::<_, Account>(
        "SELECT * FROM accounts WHERE encryption_version IS NULL OR encryption_version = 0"
    )
    .fetch_all(pool)
    .await?;

    if plaintext_accounts.is_empty() {
        info!("No plaintext accounts to migrate");
    } else {
        info!("Migrating {} accounts to encrypted storage", plaintext_accounts.len());

        for account in plaintext_accounts {
            // Encrypt tokens
            let encrypted_auth = account.encrypt_auth_data()?;
            let encrypted_refresh = account.encrypt_refresh_token()?;

            // Update database
            sqlx::query(
                "UPDATE accounts
                 SET auth_data = ?, refresh_token = ?, encryption_version = 1, encrypted_at = ?
                 WHERE id = ?"
            )
            .bind(&encrypted_auth)
            .bind(&encrypted_refresh)
            .bind(Utc::now())
            .bind(account.id)
            .execute(pool)
            .await?;
        }
    }

    // Mark migration complete
    sqlx::query("INSERT INTO schema_migrations (version, name) VALUES (3, 'encrypt_plaintext_tokens')")
        .execute(pool)
        .await?;

    Ok(())
}
```

**When**: Run after Migration 002 complete
**Idempotency**: Checks `schema_migrations` and skips if version 3 present
**Rollback**: Restore from backup (can't decrypt without original plaintext)

---

## Rollback Strategy

### Automatic Backup Before Migration

**When**: Before Migration 002 (schema changes) and Migration 003 (data encryption)

**Backup Procedure**:
```rust
async fn create_backup(db_path: &str) -> Result<PathBuf> {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let backup_path = format!("{}.backup_{}", db_path, timestamp);

    // SQLite backup (file copy is sufficient for single-file DB)
    std::fs::copy(db_path, &backup_path)
        .context("Failed to create database backup")?;

    info!("Database backup created: {}", backup_path);
    Ok(PathBuf::from(backup_path))
}
```

**Backup Location**: Same directory as `openchime.db`
**Backup Naming**: `openchime.db.backup_20251124_143000`
**Retention**: Keep last 3 backups, delete older ones

### Rollback Scenarios

#### Scenario 1: Migration 002 Fails (Schema Changes)

**Failure Modes**:
- ALTER TABLE fails (disk full, permission denied, SQLite locked)

**Automatic Rollback**:
```rust
async fn run_migration_002(pool: &SqlitePool, backup_path: &Path) -> Result<()> {
    match apply_migration_002(pool).await {
        Ok(_) => {
            info!("Migration 002 successful");
            Ok(())
        }
        Err(e) => {
            error!("Migration 002 failed: {}", e);
            restore_from_backup(backup_path, "openchime.db").await?;
            Err(anyhow!("Migration 002 failed, database restored from backup"))
        }
    }
}
```

**Recovery**: Restore backup, application continues with plaintext storage

---

#### Scenario 2: Migration 003 Fails (Token Encryption)

**Failure Modes**:
- Keystore unavailable (platform secure storage not accessible)
- Encryption fails for some accounts (corrupted data, invalid format)
- Partial migration (some accounts encrypted, some not)
- Database locked during UPDATE operations

**Automatic Rollback**:
```rust
async fn run_migration_003(pool: &SqlitePool, backup_path: &Path) -> Result<()> {
    match migrate_encrypt_tokens(pool).await {
        Ok(_) => {
            info!("Migration 003 successful");
            Ok(())
        }
        Err(e) => {
            error!("Migration 003 failed: {}", e);
            restore_from_backup(backup_path, "openchime.db").await?;
            Err(anyhow!("Migration 003 failed, database restored from backup"))
        }
    }
}
```

**Recovery**: Restore backup, application continues with plaintext storage

**User Notification**:
```
❌ Migration Failed
Database encryption could not be completed. Your database has been
restored to the previous state. OAuth tokens remain in plaintext.

Error: Failed to access platform keystore

Next Steps:
1. Check system requirements (macOS Keychain, Windows Credential Manager)
2. Restart the application
3. Contact support if issue persists
```

---

#### Scenario 3: Partial Migration (Some Accounts Encrypted)

**Failure Mode**: Migration 003 fails after encrypting some accounts but not all

**Handling**:
- **Transaction Wrapper**: Use SQLite transaction to make migration atomic
- **If Transaction Fails**: Entire migration rolls back automatically
- **If Transaction Succeeds**: All accounts encrypted or none

**Implementation**:
```rust
async fn migrate_encrypt_tokens(pool: &SqlitePool) -> Result<()> {
    // Begin transaction
    let mut tx = pool.begin().await?;

    // ... migration logic ...

    // Commit transaction (all-or-nothing)
    tx.commit().await?;

    Ok(())
}
```

---

#### Scenario 4: Key Loss After Migration

**Failure Mode**: User loses encryption key after successful migration

**This is NOT a rollback scenario** - this is a recovery scenario handled separately:

**Handling**:
- Clear error message: "Encryption key not found. Please re-authenticate your accounts."
- Application allows user to delete accounts and re-add them
- New accounts will be encrypted with new key

**Prevention**: Document platform keystore requirements in README

---

### Manual Rollback (User-Initiated)

**If automatic rollback fails**, provide manual recovery steps:

```bash
# Stop application
pkill openchime

# Restore from backup
cd ~/Library/Application\ Support/openchime  # macOS
cd ~/.local/share/openchime                  # Linux
cd %APPDATA%\openchime                       # Windows

# Find most recent backup
ls -lt openchime.db.backup_*

# Restore
cp openchime.db.backup_20251124_143000 openchime.db

# Restart application
```

**Document in**:
- README.md troubleshooting section
- Error message shown to user
- Support documentation

---

## Testing Procedure

### Test Scenarios

#### Test 1: Fresh Installation (No Migration Needed)

**Setup**: New user, no existing database

**Expected**:
1. Database created with latest schema (includes encryption_version column)
2. schema_migrations table created with versions 1, 2
3. Migration 003 skips (no plaintext accounts)
4. New accounts encrypted from the start

**Validation**:
```sql
-- Check schema_migrations
SELECT * FROM schema_migrations ORDER BY version;
-- Should show: version 1, 2

-- Check accounts table has encryption columns
PRAGMA table_info(accounts);
-- Should show: encryption_version, encrypted_at
```

---

#### Test 2: Existing Database (Migration Required)

**Setup**: Existing user with plaintext accounts

**Steps**:
1. Create test database with plaintext accounts:
   ```sql
   INSERT INTO accounts (provider, account_name, auth_data, refresh_token)
   VALUES ('google', 'test@gmail.com', 'plaintext_auth', 'plaintext_refresh');
   ```

2. Launch application with migration code

**Expected**:
1. Backup created: `openchime.db.backup_TIMESTAMP`
2. Migration 001: schema_migrations created, version 1 inserted
3. Migration 002: encryption_version, encrypted_at columns added
4. Migration 003: Plaintext account encrypted
5. Verify: auth_data and refresh_token are now encrypted (not plaintext)
6. Verify: encryption_version = 1, encrypted_at = NOW()

**Validation**:
```sql
-- Check migration status
SELECT * FROM schema_migrations;
-- Should show: versions 1, 2, 3

-- Check account is encrypted
SELECT id, encryption_version, encrypted_at,
       auth_data != 'plaintext_auth' as is_encrypted
FROM accounts WHERE account_name = 'test@gmail.com';
-- is_encrypted should be TRUE (1)
```

---

#### Test 3: Idempotency (Run Migration Twice)

**Setup**: Database with migrations already applied

**Steps**:
1. Apply all migrations (versions 1, 2, 3)
2. Restart application (migrations should run again)

**Expected**:
1. Migration 001: `INSERT OR IGNORE` skips (version 1 exists)
2. Migration 002: Column add skipped (columns exist)
3. Migration 003: `SELECT FROM schema_migrations WHERE version = 3` returns row, skips
4. No errors, no duplicate data

**Validation**:
```sql
-- Check no duplicate entries
SELECT version, COUNT(*) as count
FROM schema_migrations
GROUP BY version;
-- All counts should be 1
```

---

#### Test 4: Migration Failure and Rollback

**Setup**: Simulate encryption failure

**Steps**:
1. Create test database with plaintext account
2. Mock keystore to fail (return error)
3. Launch application

**Expected**:
1. Backup created
2. Migration 003 attempts to encrypt tokens
3. Keystore error occurs
4. Backup restored automatically
5. Application shows error message
6. Database in original plaintext state

**Validation**:
```sql
-- Migration 003 should NOT be in schema_migrations
SELECT * FROM schema_migrations WHERE version = 3;
-- Should return no rows

-- Account should still be plaintext
SELECT encryption_version FROM accounts;
-- Should be NULL
```

---

#### Test 5: Partial Migration (Transaction Rollback)

**Setup**: Database with 10 accounts, simulate failure on 5th account

**Steps**:
1. Insert 10 plaintext accounts
2. Mock encryption to fail on 5th account
3. Launch application

**Expected**:
1. Transaction begins
2. Accounts 1-4 encrypted successfully
3. Account 5 fails
4. **Transaction rolls back** (all accounts remain plaintext)
5. Backup restored
6. schema_migrations version 3 NOT inserted

**Validation**:
```sql
-- All accounts should be plaintext (none encrypted)
SELECT COUNT(*) FROM accounts WHERE encryption_version = 1;
-- Should return 0

SELECT COUNT(*) FROM accounts WHERE encryption_version IS NULL;
-- Should return 10
```

---

#### Test 6: Multiple Account Types (Google + Proton)

**Setup**: Database with both Google and Proton accounts

**Steps**:
1. Insert Google account (with refresh_token)
2. Insert Proton account (no refresh_token)
3. Launch application

**Expected**:
1. Both accounts encrypted
2. Google: auth_data + refresh_token encrypted
3. Proton: auth_data encrypted, refresh_token remains NULL
4. Both accounts: encryption_version = 1

**Validation**:
```sql
-- Check both accounts encrypted
SELECT provider,
       encryption_version,
       refresh_token IS NOT NULL as has_refresh
FROM accounts;
-- Google: encryption_version=1, has_refresh=TRUE
-- Proton: encryption_version=1, has_refresh=FALSE
```

---

### Performance Testing

**Benchmark Migration Time**:

| Accounts | Expected Time | Max Acceptable |
|----------|---------------|----------------|
| 1        | <100ms        | 500ms          |
| 10       | <500ms        | 2s             |
| 50       | <2s           | 5s             |
| 100      | <5s           | 10s            |

**Test Procedure**:
1. Create database with N accounts
2. Measure migration 003 execution time
3. Verify <5 second target for realistic workload (10-50 accounts)

---

### Integration Testing

**Test in Real Environment**:

1. **macOS**: Test with Keychain, verify prompts (if any)
2. **Windows**: Test with Credential Manager
3. **Linux**: Test with Secret Service (GNOME/KDE)
4. **Linux (no Secret Service)**: Test fallback behavior

**Verify**:
- Logs show migration progress
- No errors in console
- Application functional after migration
- Accounts still sync correctly
- OAuth tokens work (decrypt successfully)

---

## Migration Execution Flow

### Integration into Database::new()

**Current Flow** (src/database/mod.rs):
```rust
pub async fn new() -> Result<Self> {
    // 1. Create database if not exists
    // 2. Connect to database
    // 3. Run schema (run_schema)
    // 4. Ensure migrations (ensure_migrations) <- existing ad-hoc migrations
    // 5. Return Database
}
```

**New Flow** (with schema versioning):
```rust
pub async fn new() -> Result<Self> {
    // 1. Create database if not exists
    // 2. Connect to database
    // 3. Run baseline schema (run_schema)
    // 4. Ensure legacy migrations (ensure_migrations) <- keep existing for backward compat
    // 5. **NEW: Run schema migrations (run_schema_migrations)** <- new system
    //    - Migration 001: Create schema_migrations table
    //    - Migration 002: Add encryption columns
    //    - Migration 003: Encrypt tokens (with backup + rollback)
    // 6. Return Database
}
```

**New Function**:
```rust
async fn run_schema_migrations(pool: &SqlitePool) -> Result<()> {
    // Create migrations tracking table (Migration 001)
    init_migrations_table(pool).await?;

    // Migration 002: Add encryption metadata columns
    if !migration_applied(pool, 2).await? {
        let backup = create_backup("openchime.db").await?;
        match apply_migration_002(pool).await {
            Ok(_) => info!("Migration 002 applied successfully"),
            Err(e) => {
                error!("Migration 002 failed: {}", e);
                restore_from_backup(&backup, "openchime.db").await?;
                return Err(e);
            }
        }
    }

    // Migration 003: Encrypt existing plaintext tokens
    if !migration_applied(pool, 3).await? {
        let backup = create_backup("openchime.db").await?;
        match migrate_encrypt_tokens(pool).await {
            Ok(_) => info!("Migration 003 applied successfully"),
            Err(e) => {
                error!("Migration 003 failed: {}", e);
                restore_from_backup(&backup, "openchime.db").await?;
                return Err(e);
            }
        }
    }

    Ok(())
}
```

---

## Success Criteria

Migration is considered successful when:

- [x] **Functional Requirements**:
  - schema_migrations table created and tracking versions
  - accounts table has encryption_version and encrypted_at columns
  - All existing accounts migrated to encrypted storage (encryption_version = 1)
  - New accounts created with encryption_version = 1 from the start

- [x] **Safety Requirements**:
  - Automatic backup created before any schema/data changes
  - Migration is idempotent (safe to run multiple times)
  - Transaction-wrapped (atomic: all accounts encrypted or none)
  - Rollback on failure (backup restored automatically)

- [x] **Performance Requirements**:
  - Migration completes in <5 seconds for 50 accounts
  - No perceptible startup delay for already-migrated databases

- [x] **Quality Requirements**:
  - All 6 test scenarios pass
  - Integration tests pass on macOS, Windows, Linux
  - Logging sufficient for debugging migration issues
  - Error messages clear and actionable

---

## Implementation Checklist

**Design Phase** (This Document):
- [x] Migration approach decided (online, automatic)
- [x] Schema versioning system designed
- [x] Migration SQL designed (001, 002, 003)
- [x] Rollback strategy documented
- [x] Testing procedure documented

**Implementation Phase** (TASK-010):
- [ ] Create `src/database/migrations/mod.rs` module
- [ ] Implement `schema_migrations` table creation
- [ ] Implement migration tracking functions
- [ ] Implement Migration 002 (schema changes)
- [ ] Implement Migration 003 (token encryption)
- [ ] Implement backup/restore functions
- [ ] Integrate into `Database::new()`
- [ ] Add comprehensive logging

**Testing Phase** (TASK-011 + TASK-023):
- [ ] Unit tests for migration functions
- [ ] Integration tests for all 6 scenarios
- [ ] Performance benchmarks
- [ ] Platform-specific testing (macOS, Windows, Linux)

**Documentation Phase** (TASK-012):
- [ ] Update README.md with migration info
- [ ] Update schema.sql with new columns
- [ ] Document rollback procedure for users
- [ ] Add troubleshooting guide

---

## Open Questions

1. **Backup Retention**: How many backups should we keep?
   - **Proposal**: Keep last 3 backups, delete older ones (prevents disk bloat)

2. **Migration Progress UI**: Should we show migration progress to user?
   - **Proposal**: No UI for now (migration is fast <5s), just logging

3. **Downgrade Support**: What if user installs older version after migration?
   - **Proposal**: Older version shows error "Database version too new, please update OpenChime"

4. **Failed Key Access After Migration**: How to handle if user can't access keystore later?
   - **Proposal**: Clear error, prompt to delete + re-add accounts (documented in README)

---

## Appendix: Migration SQL Files

### Migration 001: Baseline

**File**: `src/database/migrations/001_create_migrations_table.sql`
```sql
-- Create migrations tracking table
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    checksum TEXT
);

-- Mark baseline schema as version 1
INSERT OR IGNORE INTO schema_migrations (version, name)
VALUES (1, 'baseline_schema');
```

---

### Migration 002: Add Encryption Metadata

**File**: `src/database/migrations/002_add_encryption_metadata.sql`
```sql
-- Add encryption_version column
ALTER TABLE accounts ADD COLUMN encryption_version INTEGER DEFAULT NULL;

-- Add encrypted_at timestamp
ALTER TABLE accounts ADD COLUMN encrypted_at DATETIME DEFAULT NULL;

-- Mark migration as complete
INSERT OR IGNORE INTO schema_migrations (version, name)
VALUES (2, 'add_encryption_metadata');
```

---

### Migration 003: Encrypt Tokens (Rust Implementation)

**File**: `src/database/migrations/003_encrypt_tokens.rs`

See pseudo-code in "Migration 003" section above. This will be a full Rust implementation in TASK-010.

---

## Document History

| Version | Date       | Changes                          | Author           |
|---------|------------|----------------------------------|------------------|
| 1.0     | 2025-11-24 | Initial migration plan created   | Development Team |

