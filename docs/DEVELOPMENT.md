# Developer Guide

## Architecture Overview

OpenChime is a local-first desktop application built with Rust.

### Core Components

*   **UI (`src/ui`, `src/ui_state`)**: Built with `iced`. The application follows The Elm Architecture (Model-View-Update).
*   **State Management (`src/app.rs`)**: `OpenChimeApp` holds the global state (`AppState`).
*   **Database (`src/database`)**: SQLite via `sqlx`. Stores accounts, events, and settings.
*   **Calendar (`src/calendar`)**: Handles ICS fetching and parsing.
    *   `common.rs`: Shared logic for HTTP requests, ICS validation, and datetime parsing.
    *   `google.rs` / `proton.rs`: Provider-specific wrappers.
*   **Alerts (`src/alerts`)**: Background monitoring for upcoming events.

### Threading Model

*   **Main Thread**: Managed by `iced` (via `winit`) for UI rendering and event handling.
*   **Async Runtime**: Explicitly managed `tokio` runtime for background tasks (database I/O, network requests).
    *   We do *not* use `#[tokio::main]` to avoid conflicts with `winit` on macOS.
    *   The runtime is initialized in `main.rs` and passed to the app or used via `tokio::spawn`.

## Setup

1.  **Install Rust**: `rustup update`
2.  **Install Dependencies**:
    *   Linux: `sudo apt install libsqlite3-dev pkg-config libssl-dev`
    *   macOS: `brew install sqlite`

## Testing

*   **Unit Tests**: `cargo test`
*   **Integration Tests**: `cargo test --test integration_database`
*   **UI Tests**: Currently manual verification.

## Common Issues

### "Database is locked"
SQLite allows only one writer at a time. Ensure you are not running multiple instances of the app or accessing the DB from an external tool while the app is running.

### "Runtime dropped" panic
Ensure `tokio` runtime is not dropped while `iced` is still running. We handle this by keeping the runtime guard in `main` until `OpenChimeApp::run` returns.
