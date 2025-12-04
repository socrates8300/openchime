# OpenChime

A cross-platform meeting reminder application built with Rust and Iced GUI framework.

## Features

- 📅 Multi-calendar support (Proton Calendar, Google Calendar via ICS)
- 🔔 Smart meeting alerts with customizable timing
- 🎵 Multiple alert sound options
- 🔒 Local-first privacy-focused design
- 🖥️ Cross-platform (Windows, macOS, Linux)

## Prerequisites

- Rust 1.70 or later
- SQLite 3

## Setup

### 1. Install Rust

Follow the instructions at [https://rustup.rs/](https://rustup.rs/)

### 2. Build and Run

```bash
# Clone the repository
git clone https://github.com/yourusername/openchime.git
cd openchime

# Build the project
cargo build --release

# Run the application
cargo run --release
```

## Configuration

Application settings can be configured through the Settings UI:
- Alert sounds (Bells, Marimba, Piano, Gentle, Chime)
- Alert timing preferences
- Account management

## Usage

### Adding a Calendar Account (ICS)

OpenChime uses standard ICS (iCalendar) feeds to sync your events. This works with Proton Calendar, Google Calendar, Outlook, and others.

1. **Get your ICS Link**:
   - **Proton Calendar**: Settings > Calendars > Select calendar > Share via link > Copy URL.
   - **Google Calendar**: Settings > Select calendar > Integrate calendar > Secret address in iCal format.
2. **Add to OpenChime**:
   - Go to Settings.
   - Enter a name for your calendar (e.g., "Work").
   - Paste the ICS URL.
   - Click "Link Account".

### Alert Behavior

- **Video meetings**: Alerts trigger 3 minutes before start time
- **Regular meetings**: Alerts trigger 1 minute before start time
- **Snooze**: Up to 3 snoozes allowed (2 minutes each)

## Security

OpenChime takes security seriously:

- ✅ **Local-first** - all data stays on your device
- ✅ **No OAuth tokens** - uses read-only ICS feeds
- ✅ **HTTPS only** - for all external calendar syncs
- ✅ **No credentials in logs** - PII-safe error handling

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test config::tests

# Run with output
cargo test -- --nocapture
```

### Project Structure

```
openchime/
├── src/
│   ├── main.rs           # Application entry point (minimal)
│   ├── app.rs            # Main application logic and state
│   ├── messages.rs       # Unified Message enum
│   ├── lib.rs            # Library exports
│   ├── config.rs         # Configuration validation
│   ├── database/         # SQLite database operations
│   ├── calendar/         # Calendar provider integrations
│   │   ├── common.rs     # Shared ICS logic (fetching, parsing)
│   │   ├── google.rs     # Google Calendar logic (ICS)
│   │   └── proton.rs     # Proton Calendar logic (ICS)
│   ├── alerts/           # Alert monitoring logic
│   ├── audio/            # Audio playback
│   ├── models/           # Data models
│   ├── ui/               # UI components
│   │   ├── styles.rs     # Custom UI styles
│   │   └── mod.rs        # UI helpers
│   └── ui_state/         # UI state management
├── docs/                 # Documentation
└── tests/                # Integration tests
```

## Troubleshooting

### Database errors on startup

**Problem:** `Failed to initialize database`

**Solution:**
- Check disk permissions in the application data directory
- Ensure SQLite is available on your system
- Delete the database file to reset: `rm ~/.local/share/openchime/openchime.db` (Linux/macOS)

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built with [Iced](https://github.com/iced-rs/iced) GUI framework
- Inspired by the Chime macOS app
