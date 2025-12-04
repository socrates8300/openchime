-- OpenChime Database Schema
-- Local-first SQLite database for calendar events and settings

-- Accounts table: Stores authentication info for calendar providers
-- Note: auth_data and refresh_token are encrypted at rest using AES-256-GCM
CREATE TABLE IF NOT EXISTS accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL CHECK (provider IN ('google', 'proton')),
    account_name TEXT NOT NULL,
    auth_data TEXT NOT NULL, -- Encrypted: OAuth tokens for Google, ICS URL for Proton
    refresh_token TEXT,      -- Encrypted: OAuth refresh token (Google only)
    last_synced_at DATETIME,
    encryption_version INTEGER DEFAULT 1, -- Tracks encryption algorithm version (1 = AES-256-GCM)
    encrypted_at DATETIME DEFAULT CURRENT_TIMESTAMP, -- When tokens were encrypted
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Events table: Local cache of calendar data with Chime-specific fields
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    external_id TEXT NOT NULL, -- UUID or Provider ID
    account_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    start_time DATETIME NOT NULL,
    end_time DATETIME NOT NULL,
    video_link TEXT,
    video_platform TEXT,
    snooze_count INTEGER DEFAULT 0,
    has_alerted BOOLEAN DEFAULT 0,
    last_alert_threshold INTEGER, -- Closest minute-threshold alerted (e.g. 30, 10, 5, 1, 0)
    is_dismissed BOOLEAN DEFAULT 0,
    last_snoozed_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

-- Settings table: User preferences and application configuration
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Insert default settings
INSERT OR IGNORE INTO settings (key, value) VALUES 
('sound', 'bells'),
('volume', '0.7'),
('video_alert_offset', '3'),
('regular_alert_offset', '1'),
('snooze_interval', '2'),
('max_snoozes', '3'),
('sync_interval', '300'),
('auto_join_enabled', 'false'),
('theme', 'dark'),
('alert_30m', 'false'),
('alert_10m', 'false'),
('alert_5m', 'true'),
('alert_1m', 'true'),
('alert_default', 'true');

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_events_start_time ON events(start_time);
CREATE INDEX IF NOT EXISTS idx_events_account_id ON events(account_id);
CREATE INDEX IF NOT EXISTS idx_events_external_id ON events(external_id);
CREATE INDEX IF NOT EXISTS idx_events_alert ON events(has_alerted, is_dismissed, start_time);
CREATE INDEX IF NOT EXISTS idx_accounts_provider ON accounts(provider);

-- Schema Migrations table: Tracks applied database migrations
-- Used by the migration system to ensure idempotent migrations
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    checksum TEXT -- Reserved for future use (migration file integrity verification)
);

-- Mark baseline schema as migration version 1
-- This allows the migration system to track schema evolution
INSERT OR IGNORE INTO schema_migrations (version, name)
VALUES (1, 'baseline_schema');
