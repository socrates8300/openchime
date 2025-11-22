# OpenChime Implementation Task List

## Project Overview
Cross-platform Rust clone of Chime app using Tauri framework and SQLite database.

## Task Breakdown

### High Priority Tasks (Core Foundation)

#### 1. Set up Tauri project structure with Rust backend and frontend
- Initialize Tauri v2 project
- Configure Cargo.toml with required dependencies
- Set up frontend framework (React/Svelte)
- Configure Tauri build settings

#### 2. Create SQLite database schema with accounts, events, and settings tables
- Implement schema.sql with accounts, events, settings tables
- Set up SQLx for database operations
- Create database migration system
- Test database connectivity

#### 3. Define Rust data structures for CalendarEvent, Account, and Settings
- Create structs matching database schema
- Implement Serde serialization/deserialization
- Add SQLx FromRow traits
- Define enums for calendar providers

#### 4. Build background polling loop with smart alert timing logic
- Implement tokio-based monitoring loop
- Add 30-second polling interval
- Implement smart timing (3 mins video, 1 min regular)
- Add event filtering logic

#### 5. Build full-screen transparent alert window with pulsating borders
- Create Tauri window configuration (transparent, always on top)
- Implement HTML/CSS for alert overlay
- Add pulsating border animation
- Implement focus-stealing behavior

#### 6. Implement local-first privacy with all data stored in SQLite
- Ensure no external data transmission
- Validate all data stays local
- Add data encryption if needed
- Document privacy guarantees

### Medium Priority Tasks (Key Features)

#### 7. Implement Google Calendar OAuth2 integration with token storage
- Set up OAuth2 flow with local server
- Implement token refresh logic
- Store tokens securely in database
- Add Google Calendar API integration

#### 8. Implement Proton Calendar ICS feed fetching and parsing
- Add ICS URL input interface
- Implement reqwest-based fetching
- Parse ICS data with icalendar crate
- Handle parsing errors gracefully

#### 9. Create video link extraction with regex for 30+ platforms
- Implement regex patterns for major platforms
- Add extraction from description and location
- Support Zoom, Teams, Meet, Webex, etc.
- Test pattern matching accuracy

#### 10. Implement Rodio audio playback for custom alert sounds
- Add Rodio dependency
- Load custom sound files
- Implement playback on alert trigger
- Add volume controls

#### 11. Create system tray menu with upcoming events and settings
- Implement Tauri system tray
- Create tray menu UI
- Show upcoming events list
- Add settings access

#### 12. Implement strict snooze logic (max 3 snoozes, 2-minute intervals)
- Add snooze counter to database
- Implement 2-minute snooze intervals
- Disable snooze after 3 attempts
- Auto-dismiss after limit

#### 13. Add Tauri commands for frontend-backend communication
- Define invoke_handler commands
- Implement account management
- Add event querying
- Handle alert interactions

#### 14. Add error handling and logging throughout the application
- Implement proper error types
- Add logging with appropriate levels
- Handle network failures gracefully
- Add user-friendly error messages

#### 15. Set up automated calendar sync every 5 minutes
- Implement sync scheduling
- Add incremental sync logic
- Handle sync conflicts
- Update local cache efficiently

#### 16. Test meeting detection and alert timing accuracy
- Create test scenarios
- Verify timing calculations
- Test edge cases (midnight, timezone)
- Validate alert triggers

### Low Priority Tasks (Polish & Distribution)

#### 17. Ensure cross-platform compatibility (Windows/Linux/macOS)
- Test on Windows (MSI installer)
- Test on Linux (libappindicator3 dependency)
- Test on macOS (permissions, .app bundle)
- Handle Wayland limitations

#### 18. Create application settings and user preferences management
- Implement settings UI
- Add sound selection
- Configure alert offsets
- Store preferences in database

#### 19. Create build and distribution pipeline for all platforms
- Set up GitHub Actions
- Configure automatic builds
- Generate installers for each platform
- Set up release process

## Implementation Order
1. **Foundation First**: Complete all high-priority tasks in order
2. **Core Features**: Implement medium-priority tasks based on dependencies
3. **Polish Phase**: Complete low-priority tasks for production readiness

## Dependencies
- Google OAuth depends on: Project setup, database schema, data models
- Proton ICS depends on: Project setup, database schema, data models
- Alert UI depends on: Project setup, monitoring loop
- Tray UI depends on: Project setup, Tauri commands

## Notes
- All tasks should maintain local-first privacy principle
- Cross-platform testing should happen throughout development
- Error handling should be considered for every component