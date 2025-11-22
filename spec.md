This is a comprehensive implementation specification for **"OpenChime"** (a working title), a cross-platform Rust clone of the Chime macOS app.

Based on the FAQ provided, this specification replicates the core "aggressive" notification philosophy, local-first privacy, and video conferencing detection, while expanding the platform support to Windows, Linux, and macOS using the **Tauri** framework and **SQLite**.

---

# OpenChime: Implementation Specification

## 1. Technology Stack

*   **Core Language:** Rust (Edition 2021)
*   **GUI Framework:** **Tauri v2**
    *   *Reasoning:* Allows for cross-platform (Windows/Linux/macOS) system tray support, transparent windows for "full-screen takeover," and HTML/CSS/JS frontend for easy styling of "pulsating borders."
*   **Database:** **SQLite** (via `sqlx` or `rusqlite`)
    *   *Reasoning:* mandated by prompt; ensures local-only data storage as per Chime's privacy policy.
*   **Async Runtime:** **Tokio**
*   **Audio:** **Rodio** (for playing the 5 custom alert sounds).
*   **Calendar Parsing:** `icalendar` (for Proton/ICS) and `google-calendar` API crates.

---

## 2. Architecture Overview

The application consists of two main processes managed by Tauri:

1.  **The Background Sentinel (Rust):**
    *   Runs the polling loop (every 60 seconds).
    *   Manages OAuth tokens (Google) and ICS fetching (Proton).
    *   Interacts with the SQLite database.
    *   Calculates "Smart Alert Timing" (3 mins for video, 1 min for regular).
2.  **The Frontend (Web Tech/React/Svelte):**
    *   **Tray Window:** Small list of upcoming events/stats.
    *   **Alert Window:** Full-screen transparent overlay that forces focus.

---

## 3. Database Schema (SQLite)

We need to store account credentials, cached events, and local state (snooze counts).

```sql
-- 1. Accounts: Stores auth info for Google and Proton
CREATE TABLE accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL, -- 'google' or 'proton' (via ics)
    account_name TEXT NOT NULL,
    auth_data TEXT NOT NULL, -- JSON: OAuth tokens for Google, ICS URL for Proton
    refresh_token TEXT,
    last_synced_at DATETIME
);

-- 2. Events: Local cache of calendar data
CREATE TABLE events (
    id TEXT PRIMARY KEY, -- UUID or Provider ID
    account_id INTEGER,
    title TEXT NOT NULL,
    start_time DATETIME NOT NULL,
    end_time DATETIME NOT NULL,
    video_link TEXT, -- Extracted via Regex
    video_platform TEXT, -- 'zoom', 'teams', etc.
    is_all_day BOOLEAN DEFAULT 0,
    
    -- Chime specific logic
    snooze_count INTEGER DEFAULT 0, -- Limit to 3
    has_alerted BOOLEAN DEFAULT 0,
    is_dismissed BOOLEAN DEFAULT 0,
    
    FOREIGN KEY(account_id) REFERENCES accounts(id)
);

-- 3. Settings: User preferences
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT
);
-- Defaults to insert: 
-- ('sound', 'bells'), ('video_alert_offset', '3'), ('regular_alert_offset', '1')
```

---

## 4. Rust Data Structures

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub video_link: Option<String>,
    pub video_platform: Option<String>, // Zoom, Teams, Meet
    pub snooze_count: i32,
    pub has_alerted: bool,
}

pub enum CalendarProvider {
    Google,
    Proton, // Implemented via ICS subscription
}
```

---

## 5. Integration Logic

### A. Google Calendar (OAuth2)
Since Google has a REST API, we will use the `oauth2` crate.
1.  **Setup:** Register a GCP project to get Client ID/Secret.
2.  **Flow:** Spin up a local server on `localhost` to catch the OAuth callback.
3.  **Sync:** Fetch `events.list`. Parse JSON response. Store in SQLite.

### B. Proton Calendar (The Challenge)
Proton Calendar is end-to-end encrypted. A third-party app cannot easily decrypt the data without implementing the full Proton PGP stack (which is heavy and complex).
**Solution:** Use Proton's **"Share via Link"** feature.
1.  User generates a secret ICS link in Proton Calendar settings.
2.  User pastes this link into OpenChime.
3.  **Rust Logic:** Use `reqwest` to fetch the `.ics` file and the `icalendar` crate to parse it. This is read-only, which fits the Chime model (Chime reads calendars, it doesn't edit them heavily except for local state).

### C. Video Link Extraction (Regex)
Chime detects 30+ platforms. We implement this in Rust using Regex during the sync phase.

```rust
pub fn extract_video_link(description: &str, location: &str) -> Option<(String, String)> {
    // Simplified Regex patterns for common platforms
    let patterns = vec![
        (r"https://.*zoom\.us/j/\d+", "Zoom"),
        (r"https://meet\.google\.com/[a-z-]+", "Google Meet"),
        (r"https://teams\.microsoft\.com/l/meetup-join/.*", "Teams"),
        // Add Webex, Discord, etc.
    ];

    let combined_text = format!("{} {}", description, location);
    
    for (pattern, name) in patterns {
        if let Some(mat) = regex::Regex::new(pattern).unwrap().find(&combined_text) {
            return Some((mat.as_str().to_string(), name.to_string()));
        }
    }
    None
}
```

---

## 6. The "Chime" Logic (Polling & Alerts)

This is the core loop running in a separate thread using `tokio::spawn`.

**Logic Flow:**
1.  Wake up every 30 seconds.
2.  Query SQLite for events starting in the next 5 minutes where `has_alerted == false` and `is_dismissed == false`.
3.  **Determine Trigger Time:**
    *   If `video_link` is present: Trigger if `now >= start_time - 3 minutes`.
    *   If no video link: Trigger if `now >= start_time - 1 minute`.
4.  **Fire Alert:**
    *   Play sound (via `rodio`).
    *   Invoke Tauri command to show the **Alert Window**.

**Snooze Logic (Strict):**
*   User clicks Snooze.
*   Rust updates DB: `snooze_count += 1`.
*   Rust calculates new alert time: `now + 2 minutes`.
*   **Constraint:** If `snooze_count >= 3`, disable the Snooze button in the UI and mark as "Joined/Dismissed" automatically after the timer expires.

---

## 7. Frontend / UI Specification (Tauri)

### A. The Menu Bar (Tray)
*   **Framework:** React or vanilla HTML/JS.
*   **Content:**
    *   Countdown timer to next event.
    *   List of today's meetings.
    *   "Refresh" button.
    *   Settings (gear icon).
*   **Window Behavior:** Hidden by default, toggles on tray icon click.

### B. The Alert Window (The "Intrusive" Part)
This is what makes Chime unique.
*   **Window Config (Tauri `tauri.conf.json`):**
    ```json
    {
      "fullscreen": false,
      "transparent": true,
      "alwaysOnTop": true,
      "decorations": false,
      "skipTaskbar": true
    }
    ```
*   **Visuals:**
    *   Background: Semi-transparent dark overlay (dim the rest of the screen).
    *   Center Card: Meeting Title, Countdown, Big Green "Join" Button.
    *   **CSS Animation:** A pulsating red border (`@keyframes pulse { ... }`) around the card if within 1 minute of start.
*   **Behavior:**
    *   When triggered, it steals focus.
    *   It cannot be closed via "X". Only "Join", "Dismiss", or "Snooze".

---

## 8. Rust Implementation Skeleton

Here is how the main entry point and sync logic would look.

```rust
// src/main.rs

use tauri::{Manager, SystemTray, SystemTrayEvent};
use sqlx::sqlite::SqlitePool;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

struct AppState {
    db: SqlitePool,
}

#[tokio::main]
async fn main() {
    // 1. Init Database
    let db_url = "sqlite://chime_clone.db?mode=rwc";
    let pool = SqlitePool::connect(db_url).await.unwrap();
    
    // Run migrations (create tables defined in section 3)
    sqlx::query(include_str!("schema.sql")).execute(&pool).await.unwrap();

    let app_state = Arc::new(AppState { db: pool.clone() });

    // 2. Spawn Background Sentinel
    let state_clone = app_state.clone();
    tokio::spawn(async move {
        monitor_meetings(state_clone).await;
    });

    // 3. Build Tauri App
    tauri::Builder::default()
        .manage(app_state)
        .system_tray(SystemTray::new())
        .on_system_tray_event(|app, event| {
             // Handle tray clicks to show the schedule window
        })
        .invoke_handler(tauri::generate_handler![
            add_google_account, 
            add_proton_ics, 
            snooze_meeting, 
            join_meeting
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// The Core Logic Loop
async fn monitor_meetings(state: Arc<AppState>) {
    loop {
        let now = chrono::Utc::now();
        
        // Fetch upcoming events from DB
        let upcoming = sqlx::query_as::<_, CalendarEvent>(
            "SELECT * FROM events WHERE start_time > ? AND start_time < ? AND has_alerted = 0"
        )
        .bind(now)
        .bind(now + chrono::Duration::minutes(5))
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        for event in upcoming {
            let minutes_until = (event.start_time - now).num_minutes();
            
            // Logic: 3 mins for video, 1 min for regular
            let threshold = if event.video_link.is_some() { 3 } else { 1 };

            if minutes_until <= threshold {
                trigger_alert(&event);
                
                // Mark as alerted in DB
                sqlx::query("UPDATE events SET has_alerted = 1 WHERE id = ?")
                    .bind(&event.id)
                    .execute(&state.db)
                    .await
                    .ok();
            }
        }

        // Sync Logic: Every 5 minutes, pull fresh data from Google/Proton
        if now.timestamp() % 300 == 0 {
             sync_calendars(&state.db).await;
        }

        sleep(Duration::from_secs(10)).await;
    }
}

fn trigger_alert(event: &CalendarEvent) {
    // 1. Play Sound (e.g., Marimba)
    // 2. Emit event to Tauri Frontend to open the "Alert Window"
    // tauri_app_handle.emit_all("show-alert", event).unwrap();
}
```

## 9. Platform Specific Considerations

To ensure "All Computers" (Cross-platform) compatibility:

1.  **Windows:**
    *   The Tauri installer (MSI) works out of the box.
    *   `AppLocalData` directory is standard.
2.  **Linux:**
    *   Requires `libappindicator3` for the system tray.
    *   **Wayland Warning:** On modern Linux (Wayland), windows cannot programmatically steal focus or position themselves absolutely on top easily. The app may behave more like a standard notification or require specific compositor rules. The spec recommends testing on X11 first, or using standard desktop notifications (`notify-rust`) as a fallback if the full-screen window fails.
3.  **macOS:**
    *   Requires permissions in `Info.plist` for Notifications.
    *   Tauri handles the `.app` bundle generation.

## 10. Summary of Deliverables

1.  **Rust Backend:** Handles OAuth, ICS parsing, SQLite storage, and the polling timer.
2.  **Tauri Frontend:** HTML/CSS UI for the Tray and the Full-screen Alert.
3.  **Integrations:** Google (API) and Proton (ICS feed).
4.  **Privacy:** All data stays in the local SQLite file (`chime_clone.db`).
