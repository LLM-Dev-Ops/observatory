//! Data validation for storage models.
//!
//! This module provides a validation framework for all storage models,
//! ensuring data integrity before insertion into the database.

use crate::error::StorageResult;

/// Trait for validating data models before storage.
///
/// All storage models should implement this trait to ensure
/// data integrity and catch errors early.
pub trait Validate {
    /// Validate the model, returning an error with detailed field information if invalid.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::ValidationError` with a descriptive message if validation fails.
    fn validate(&self) -> StorageResult<()>;
}

/// Validates that a string is a valid hex string with the expected length.
///
/// # Arguments
///
/// * `value` - The string to validate
/// * `expected_len` - Expected length in characters (e.g., 16 for 8-byte hex, 32 for 16-byte hex)
/// * `field_name` - Name of the field for error messages
///
/// # Returns
///
/// `Ok(())` if valid, `Err` with a descriptive message if invalid
pub fn validate_hex_string(
    value: &str,
    expected_len: usize,
    field_name: &str,
) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{} cannot be empty", field_name));
    }

    if value.len() != expected_len {
        return Err(format!(
            "{} must be {} characters long, got {}",
            field_name,
            expected_len,
            value.len()
        ));
    }

    if !value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!(
            "{} must contain only hexadecimal characters (0-9, a-f, A-F), got: {}",
            field_name, value
        ));
    }

    Ok(())
}

/// Validates that a string is not empty.
///
/// # Arguments
///
/// * `value` - The string to validate
/// * `field_name` - Name of the field for error messages
///
/// # Returns
///
/// `Ok(())` if valid, `Err` with a descriptive message if invalid
pub fn validate_not_empty(value: &str, field_name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{} cannot be empty", field_name));
    }
    Ok(())
}

/// Validates that a value is within a specified range.
///
/// # Arguments
///
/// * `value` - The value to validate
/// * `min` - Minimum allowed value (inclusive)
/// * `max` - Maximum allowed value (inclusive)
/// * `field_name` - Name of the field for error messages
///
/// # Returns
///
/// `Ok(())` if valid, `Err` with a descriptive message if invalid
pub fn validate_range<T: PartialOrd + std::fmt::Display>(
    value: T,
    min: T,
    max: T,
    field_name: &str,
) -> Result<(), String> {
    if value < min || value > max {
        return Err(format!(
            "{} must be between {} and {}, got {}",
            field_name, min, max, value
        ));
    }
    Ok(())
}

/// Validates that an f64 value is not NaN or infinite.
///
/// # Arguments
///
/// * `value` - The value to validate
/// * `field_name` - Name of the field for error messages
///
/// # Returns
///
/// `Ok(())` if valid, `Err` with a descriptive message if invalid
pub fn validate_finite_f64(value: f64, field_name: &str) -> Result<(), String> {
    if value.is_nan() {
        return Err(format!("{} cannot be NaN", field_name));
    }
    if value.is_infinite() {
        return Err(format!("{} cannot be infinite", field_name));
    }
    Ok(())
}

/// Validates that one value is greater than or equal to another.
///
/// # Arguments
///
/// * `value1` - The first value (should be >= value2)
/// * `value2` - The second value
/// * `field1_name` - Name of the first field
/// * `field2_name` - Name of the second field
///
/// # Returns
///
/// `Ok(())` if valid, `Err` with a descriptive message if invalid
pub fn validate_ordering<T: PartialOrd + std::fmt::Display>(
    value1: &T,
    value2: &T,
    field1_name: &str,
    field2_name: &str,
) -> Result<(), String> {
    if value1 < value2 {
        return Err(format!(
            "{} ({}) must be >= {} ({})",
            field1_name, value1, field2_name, value2
        ));
    }
    Ok(())
}

/// Validates that a status string is one of the allowed values.
///
/// # Arguments
///
/// * `status` - The status string to validate
/// * `allowed_values` - Slice of allowed status values
/// * `field_name` - Name of the field for error messages
///
/// # Returns
///
/// `Ok(())` if valid, `Err` with a descriptive message if invalid
pub fn validate_status(
    status: &str,
    allowed_values: &[&str],
    field_name: &str,
) -> Result<(), String> {
    if !allowed_values.contains(&status) {
        return Err(format!(
            "{} must be one of [{}], got: {}",
            field_name,
            allowed_values.join(", "),
            status
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_hex_string_valid() {
        assert!(validate_hex_string("0123456789abcdef", 16, "test_field").is_ok());
        assert!(validate_hex_string("ABCDEF0123456789", 16, "test_field").is_ok());
        assert!(validate_hex_string("0123456789abcdef0123456789abcdef", 32, "test_field").is_ok());
    }

    #[test]
    fn test_validate_hex_string_empty() {
        let result = validate_hex_string("", 16, "test_field");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_hex_string_wrong_length() {
        let result = validate_hex_string("abc", 16, "test_field");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be 16 characters long"));
    }

    #[test]
    fn test_validate_hex_string_invalid_chars() {
        let result = validate_hex_string("xyz123456789abcd", 16, "test_field");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("hexadecimal characters"));
    }

    #[test]
    fn test_validate_not_empty_valid() {
        assert!(validate_not_empty("valid", "test_field").is_ok());
        assert!(validate_not_empty("  valid  ", "test_field").is_ok());
    }

    #[test]
    fn test_validate_not_empty_invalid() {
        assert!(validate_not_empty("", "test_field").is_err());
        assert!(validate_not_empty("   ", "test_field").is_err());
    }

    #[test]
    fn test_validate_range_valid() {
        assert!(validate_range(5, 0, 10, "test_field").is_ok());
        assert!(validate_range(0, 0, 10, "test_field").is_ok());
        assert!(validate_range(10, 0, 10, "test_field").is_ok());
    }

    #[test]
    fn test_validate_range_invalid() {
        let result = validate_range(-1, 0, 10, "test_field");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be between"));

        let result = validate_range(11, 0, 10, "test_field");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_finite_f64_valid() {
        assert!(validate_finite_f64(0.0, "test_field").is_ok());
        assert!(validate_finite_f64(123.456, "test_field").is_ok());
        assert!(validate_finite_f64(-123.456, "test_field").is_ok());
    }

    #[test]
    fn test_validate_finite_f64_nan() {
        let result = validate_finite_f64(f64::NAN, "test_field");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be NaN"));
    }

    #[test]
    fn test_validate_finite_f64_infinite() {
        let result = validate_finite_f64(f64::INFINITY, "test_field");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be infinite"));

        let result = validate_finite_f64(f64::NEG_INFINITY, "test_field");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_ordering_valid() {
        assert!(validate_ordering(&10, &5, "field1", "field2").is_ok());
        assert!(validate_ordering(&5, &5, "field1", "field2").is_ok());
    }

    #[test]
    fn test_validate_ordering_invalid() {
        let result = validate_ordering(&5, &10, "field1", "field2");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be >="));
    }

    #[test]
    fn test_validate_status_valid() {
        let allowed = &["ok", "error", "unset"];
        assert!(validate_status("ok", allowed, "status").is_ok());
        assert!(validate_status("error", allowed, "status").is_ok());
        assert!(validate_status("unset", allowed, "status").is_ok());
    }

    #[test]
    fn test_validate_status_invalid() {
        let allowed = &["ok", "error", "unset"];
        let result = validate_status("invalid", allowed, "status");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be one of"));
    }
}
