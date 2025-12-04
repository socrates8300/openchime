//! Configuration validation module
//!
//! Minimal validation for ICS-only calendar integration.
//! OAuth validation removed - app now uses simple ICS URLs only.

use crate::error::AppResult;
use log::info;

/// Validates basic application configuration
///
/// With OAuth removed, this now performs minimal validation.
/// Calendar accounts are validated when added by users.
///
/// # Returns
///
/// * `Ok(())` - validation always passes for now
///
pub fn validate_config() -> AppResult<()> {
    info!("Configuration validation (ICS-only mode)");
    // No OAuth credentials needed - ICS URLs are provided per-account
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_passes() {
        let result = validate_config();
        assert!(result.is_ok());
    }
}
