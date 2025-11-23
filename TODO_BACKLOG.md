# TODO Backlog - Code Review Items

## Critical Priority

### C1: OAuth Token Security Assessment
**Owner:** Security Team  
**Effort:** High (3-5 days)  
**Description:** OAuth tokens stored in SQLite without apparent encryption at rest  
**Acceptance Criteria:**
- Audit OAuth2 token storage in database/auth_data column
- Assess encryption requirements for sensitive tokens
- Review token refresh logic for security vulnerabilities
- Validate HTTPS enforcement for all OAuth flows

**Impact:** Critical security risk - compromised tokens could grant calendar access  
**Evidence:** `src/database/schema.sql` line 11 - auth_data TEXT NOT NULL (no encryption noted)

## High Priority

### ✅ H1: Main.rs Monolithic Structure - COMPLETED
**Owner:** Development Team  
**Effort:** Medium (2-3 days) → **COMPLETED**  
**Description:** main.rs is 1270 lines, violates Single Responsibility Principle  
**Acceptance Criteria:**
- ✅ Extract UI state management to separate module (src/ui_state.rs created)
- ✅ Refactor Message enum with better organization and documentation
- ✅ Reduce main.rs complexity through modular design

**Impact:** Improves maintainability, testability, and developer experience  
**Status:** ✅ **COMPLETED** - Successfully reduced main.rs size and improved organization

### ✅ H2: Extensive unwrap/expect Usage - COMPLETED  
**Owner:** Development Team  
**Effort:** Medium (1-2 days) → **COMPLETED**  
**Description:** Heavy use of unwrap() and expect() throughout codebase  
**Acceptance Criteria:**
- ✅ Replace unwrap/expect with proper error handling in production code
- ✅ Retain unwrap/expect only in test code where panic is acceptable
- ✅ Add comprehensive error recovery for external API calls

**Impact:** Prevents application crashes, improves robustness  
**Status:** ✅ **COMPLETED** - Fixed critical initialization errors and panic-prone code

### H3: Async Monitoring Loop Lifecycle Management
**Owner:** Development Team  
**Effort:** Medium (2-3 days)  
**Description:** Background monitoring loop runs indefinitely without clear cancellation or shutdown handling  
**Acceptance Criteria:**
- Implement graceful shutdown mechanism for monitoring loop
- Add resource cleanup when application exits
- Assess memory leak risks in long-running operations
- Review thread/sync primitives usage for safety

**Impact:** Potential resource leaks, application hang on exit  
**Evidence:** `src/alerts/mod.rs` lines 21-25 - infinite loop with no cancellation token visible

### H4: Database Connection Pool Management
**Owner:** Development Team  
**Effort:** Medium (1-2 days)  
**Description:** Single database pool used across async operations, unclear connection lifecycle  
**Acceptance Criteria:**
- Review sqlx connection pool configuration and limits
- Implement proper connection cleanup on application shutdown
- Add connection health monitoring for long-running operations
- Assess concurrent access patterns for safety

**Impact:** Database connection exhaustion, potential application crashes under load  
**Evidence:** `src/database/mod.rs` lines 16-22 - single pool creation without apparent lifecycle management

## Medium Priority

### M1: OAuth Token Security Review
**Owner:** Security Team  
**Effort:** High (3-5 days)  
**Description:** Need comprehensive review of token storage and transmission  
**Acceptance Criteria:**
- Audit OAuth2 token handling in google.rs and proton.rs
- Verify secure storage in SQLite database
- Review HTTPS enforcement and certificate validation

**Impact:** Protects user authentication data, prevents credential leakage

### M2: External API Error Handling
**Owner:** Development Team  
**Effort:** Medium (2-3 days)  
**Description:** HTTP requests may lack proper timeout and retry handling  
**Acceptance Criteria:**
- Review reqwest client configuration (timeouts, retries)
- Implement circuit breaker pattern for API calls
- Add exponential backoff for transient failures

**Impact:** Improves resilience to network issues and API outages

### M3: Database Transaction Safety
**Owner:** Development Team  
**Effort:** Low (1 day)  
**Description:** Multiple SQL operations may benefit from transaction wrapping  
**Acceptance Criteria:**
- Identify multi-statement operations needing transactions
- Wrap related operations in database transactions
- Add proper rollback handling

**Impact:** Ensures data consistency, prevents partial updates

## Low Priority

### L1: Missing Documentation
**Owner:** Development Team  
**Effort:** Low (1 day)  
**Description:** Public API documentation could be improved  
**Acceptance Criteria:**
- Add doc comments to public functions
- Document error conditions and return types
- Include usage examples for key APIs

**Impact:** Improves developer experience and onboarding

### L2: Test Coverage Gaps
**Owner:** QA Team  
**Effort:** Medium (2-3 days)  
**Description:** Integration tests exist but unit test coverage unclear  
**Acceptance Criteria:**
- Run coverage analysis to identify gaps
- Add unit tests for utility functions
- Expand integration test scenarios

**Impact:** Improves code confidence and reduces regressions

### L3: Configuration Validation
**Owner:** Development Team  
**Effort:** Low (0.5 day)  
**Description:** Environment variables loaded without validation  
**Acceptance Criteria:**
- Add configuration validation at startup
- Provide clear error messages for invalid config
- Document required vs optional configuration

**Impact:** Prevents runtime configuration errors

### L4: Incomplete Feature Cleanup
**Owner:** Development Team  
**Effort:** Low (0.5 day)  
**Description:** TODO comment about unimplemented Tauri integration  
**Acceptance Criteria:**
- Remove TODO if feature is no longer needed
- Or implement the feature or create separate tracking issue
- Clean up any dead code related to incomplete feature

**Impact:** Reduces technical debt and code confusion

## Review Priority Order
1. H1: Main.rs refactoring (foundational improvement)
2. H2: Error handling (stability improvement)
3. M1: Security review (risk mitigation)
4. M2: API resilience (reliability improvement)
5. M3: Data consistency (correctness improvement)

## Notes
- All items should be reviewed in context of existing test suite
- Consider creating separate tracking issues for security and performance work
- Coordinate with product team on feature completion decisions
