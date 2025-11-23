# TODO Backlog - Code Review Items

## Critical Priority
*None identified in initial analysis*

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
