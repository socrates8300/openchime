# OpenChime Testing Guide

This document describes the comprehensive testing strategy for OpenChime, a cross-platform meeting reminder application built with Rust and Tauri.

## Testing Pyramid

OpenChime follows a traditional testing pyramid with emphasis on fast, reliable unit tests:

- **Unit Tests (70%)**: Fast tests for individual functions and methods
- **Integration Tests (20%)**: Tests for component interactions and external boundaries
- **End-to-End Tests (10%)**: Full application workflow tests

## Test Organization

### Unit Tests (`src/**/*.rs`)

Unit tests are co-located with the code they test using `#[cfg(test)]` modules:

- `src/models/mod.rs` - Model methods, validation, business logic
- `src/database/mod.rs` - Database operations, queries, migrations
- `src/audio/mod.rs` - Audio system functionality, volume control
- `src/alerts/mod.rs` - Alert timing logic, event processing

### Integration Tests (`tests/integration_*.rs`)

Integration tests verify component interactions:

- `tests/integration_database.rs` - Database workflows, concurrent access
- `tests/integration_audio.rs` - Audio system with real file operations
- `tests/integration_alerts.rs` - Alert workflows with database and audio

### End-to-End Tests (`tests/e2e_*.rs`)

E2E tests verify complete application workflows:

- `tests/e2e_app_workflows.rs` - Application startup, database initialization, configuration

## Running Tests

### Quick Start

```bash
# Run all tests
./test.sh

# Run specific test categories
./test.sh unit          # Unit tests only
./test.sh integration   # Integration tests only
./test.sh e2e           # E2E tests only
```

### Manual Cargo Commands

```bash
# Run all tests
cargo test --all

# Run unit tests only
cargo test --lib

# Run integration tests only
cargo test --test '*'

# Run E2E tests only (marked as ignored)
cargo test --test e2e_app_workflows -- --ignored

# Run tests with output
cargo test --all -- --nocapture

# Run tests with specific logging
RUST_LOG=debug cargo test --all -- --nocapture
```

### Test Categories

#### Unit Tests
- **Location**: `src/**/*.rs` in `#[cfg(test)]` modules
- **Speed**: < 1 second total
- **Dependencies**: Minimal, mostly in-memory
- **When to run**: On every commit, during development

#### Integration Tests
- **Location**: `tests/integration_*.rs`
- **Speed**: 1-10 seconds
- **Dependencies**: Database, file system, audio subsystem
- **When to run**: Before pull requests, during CI

#### E2E Tests
- **Location**: `tests/e2e_*.rs`
- **Speed**: 10-60 seconds
- **Dependencies**: Full application, GUI environment
- **When to run**: Before releases, during CI

## Test Environment Setup

### Required Dependencies

```bash
# Install test dependencies
cargo install cargo-tarpaulin  # For coverage
cargo install cargo-watch      # For watch mode
cargo install cargo-audit      # For security audit
```

### Environment Variables

```bash
export OPENCHIME_TEST_MODE="true"      # Enable test mode
export OPENCHIME_DB_PATH="test.db"     # Test database path
export RUST_LOG="debug"                # Enable debug logging
export RUST_BACKTRACE="1"              # Enable backtraces
```

## Test Coverage

### Generating Coverage Reports

```bash
# Generate HTML coverage report
./test.sh coverage

# View coverage report
open coverage/tarpaulin-report.html
```

### Coverage Goals

- **Models**: 95%+ coverage
- **Database**: 90%+ coverage  
- **Audio**: 85%+ coverage
- **Alerts**: 90%+ coverage
- **Overall**: 90%+ coverage

## Test Data Management

### Temporary Files

Tests use `tempfile` crate for temporary databases and files:

```rust
use tempfile::NamedTempFile;

let temp_file = NamedTempFile::new().unwrap();
let db_path = format!("sqlite:file:{}?mode=rwc", temp_file.path().to_str().unwrap());
```

### Test Isolation

- Each test gets its own temporary database
- Tests run in parallel where possible
- `serial_test` crate used for tests requiring sequential execution

## Mocking and Test Doubles

### External APIs

Calendar APIs (Google, Proton) are mocked using `mockall`:

```rust
use mockall::mock;

mock! {
    CalendarClient {
        async fn sync_events(&self) -> Result<Vec<CalendarEvent>, Error>;
    }
}
```

### Audio System

Audio playback is tested with mock sound files and fallback to sine waves:

```rust
// Tests use minimal WAV files or default sine wave
let sound_files = SoundFiles {
    meeting_alert: temp_dir.path().join("meeting.wav"),
    // ...
};
```

## Continuous Integration

### GitHub Actions Workflow

```yaml
name: Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run tests
        run: ./test.sh all
      - name: Generate coverage
        run: ./test.sh coverage
```

### Test Matrix

- **OS**: Ubuntu, macOS, Windows
- **Rust**: Stable, Beta (optional)
- **Features**: Default, minimal (optional)

## Debugging Tests

### Common Issues

1. **Database Lock Errors**: Ensure tests use unique temporary files
2. **Audio System Errors**: Tests should handle missing audio gracefully
3. **GUI Tests**: Mark E2E tests as `#[ignore]` for CI environments

### Debug Commands

```bash
# Run with debug output
RUST_LOG=debug cargo test --all -- --nocapture

# Run specific test with output
cargo test test_name -- --exact --nocapture

# Run tests and show backtraces
RUST_BACKTRACE=1 cargo test --all

# Run tests in single thread (for debugging race conditions)
cargo test --all -- --test-threads=1
```

## Best Practices

### Writing Tests

1. **Arrange-Act-Assert**: Structure tests clearly
2. **Descriptive Names**: Test names should describe what they test
3. **One Assertion**: Prefer one assertion per test when possible
4. **Test Edge Cases**: Test boundaries, empty inputs, error conditions
5. **Avoid Sleeps**: Use synchronization primitives instead of `sleep()`

### Test Data

1. **Minimal Setup**: Use the smallest test data needed
2. **Realistic Data**: Test with realistic but simple data
3. **Cleanup**: Ensure tests clean up resources
4. **Isolation**: Tests should not depend on each other

### Performance

1. **Fast Tests**: Unit tests should run in milliseconds
2. **Parallel Execution**: Use `tokio::test` for async tests
3. **Resource Sharing**: Share expensive resources between tests when possible
4. **Test Selection**: Run only relevant tests during development

## Troubleshooting

### Common Test Failures

1. **Permission Denied**: Check file permissions for test directories
2. **Database Locked**: Ensure proper cleanup in tests
3. **Audio Device Error**: Tests should handle missing audio gracefully
4. **Network Timeouts**: Mock external dependencies in tests

### Getting Help

- Check test output with `--nocapture`
- Use `RUST_LOG=debug` for detailed logging
- Run tests sequentially with `--test-threads=1`
- Check CI logs for environment-specific issues

## Future Improvements

### Planned Enhancements

1. **Property-Based Testing**: Add `proptest` for random test generation
2. **Fuzz Testing**: Add fuzz testing for parsing functions
3. **Performance Tests**: Add benchmarks for critical paths
4. **Visual Testing**: Add UI testing for Tauri frontend
5. **Mutation Testing**: Add mutation testing with `cargo mutants`

### Test Metrics

Track these metrics over time:
- Test execution time
- Test coverage percentage
- Test failure rate
- Number of tests per module
- Integration test reliability