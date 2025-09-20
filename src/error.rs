//! # Error Handling
//! 
//! Comprehensive error system for NOML parsing, validation, and manipulation.
//! Designed for clarity, debuggability, and extensibility.

use std::io;
use thiserror::Error;

/// The main result type used throughout NOML operations.
pub type Result<T> = std::result::Result<T, NomlError>;

/// Comprehensive error types for all NOML operations.
/// 
/// This error system is designed to provide maximum clarity about what went wrong,
/// where it happened, and how to fix it. Each variant includes enough context
/// for both developers and end users to understand and resolve issues.
#[derive(Error, Debug)]
pub enum NomlError {
    #[error("Parse error at line {line}, column {column}: {message}")]
    /// Parsing errors - when the input cannot be parsed due to syntax issues.
    Parse {
        /// Human-readable error message
        message: String,
        /// Line number where error occurred (1-indexed)
        line: usize,
        /// Column number where error occurred (1-indexed) 
        column: usize,
        /// Optional source code snippet showing the error
        snippet: Option<String>,
    },

    /// Validation errors - when NOML is syntactically correct but semantically invalid
    #[error("Validation error: {message}")]
    Validation {
        /// Description of the validation failure
        message: String,
        /// Path to the invalid key/section
        path: Option<String>,
    },

    /// Key access errors - when requesting non-existent keys
    #[error("Key '{key}' not found")]
    KeyNotFound {
        /// The key that was requested
        key: String,
        /// Available keys at that level (for suggestions)
        available: Vec<String>,
    },

    /// Type conversion errors - when values cannot be converted to requested type
    #[error("Type error: cannot convert '{value}' to {expected_type}")]
    Type {
        /// The value that couldn't be converted
        value: String,
        /// The expected type
        expected_type: String,
        /// The actual type found
        actual_type: String,
    },

    /// File I/O errors - wraps std::io::Error with additional context
    #[error("File error for '{path}': {source}")]
    Io {
        /// Path to the file that caused the error
        path: String,
        /// The underlying I/O error
        #[source]
        source: io::Error,
    },

    /// Variable interpolation errors
    #[error("Interpolation error: {message}")]
    Interpolation {
        /// Description of the interpolation failure
        message: String,
        /// The variable expression that failed
        expression: String,
        /// Path in the document where this occurred
        context: Option<String>,
    },

    /// Environment variable errors
    #[error("Environment variable '{var}' is not set")]
    EnvVar {
        /// Name of the missing environment variable
        var: String,
        /// Whether a default was expected
        has_default: bool,
    },

    /// Import/include file errors
    #[error("Import error: failed to import '{path}': {reason}")]
    Import {
        /// Path that failed to import
        path: String,
        /// Reason for import failure
        reason: String,
        /// Path of the file that tried to do the import
        from: Option<String>,
    },

    /// Schema validation errors
    #[error("Schema error at '{path}': {message}")]
    Schema {
        /// Path where schema validation failed
        path: String,
        /// Description of the schema violation
        message: String,
        /// Expected schema type/format
        expected: Option<String>,
    },

    /// Circular reference errors (for imports and references)
    #[error("Circular reference detected: {chain}")]
    CircularReference {
        /// The chain of references that caused the cycle
        chain: String,
    },

    /// Internal errors - these should never happen in normal operation
    #[error("Internal error: {message}")]
    Internal {
        /// Description of the internal error
        message: String,
        /// Optional context about where this occurred
        context: Option<String>,
    },
}

impl NomlError {
    /// Create a parse error with position information
    pub fn parse(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self::Parse {
            message: message.into(),
            line,
            column,
            snippet: None,
        }
    }

    /// Create a parse error with source code snippet
    pub fn parse_with_snippet(
        message: impl Into<String>,
        line: usize,
        column: usize,
        snippet: impl Into<String>,
    ) -> Self {
        Self::Parse {
            message: message.into(),
            line,
            column,
            snippet: Some(snippet.into()),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            path: None,
        }
    }

    /// Create a validation error with path context
    pub fn validation_at(message: impl Into<String>, path: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            path: Some(path.into()),
        }
    }

    /// Create a key not found error
    pub fn key_not_found(key: impl Into<String>) -> Self {
        Self::KeyNotFound {
            key: key.into(),
            available: Vec::new(),
        }
    }

    /// Create a key not found error with suggestions
    pub fn key_not_found_with_suggestions(
        key: impl Into<String>,
        available: Vec<String>,
    ) -> Self {
        Self::KeyNotFound {
            key: key.into(),
            available,
        }
    }

    /// Create a type conversion error
    pub fn type_error(
        value: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self::Type {
            value: value.into(),
            expected_type: expected.into(),
            actual_type: actual.into(),
        }
    }

    /// Create an I/O error with path context
    pub fn io(path: impl Into<String>, error: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source: error,
        }
    }

    /// Create an interpolation error
    pub fn interpolation(message: impl Into<String>, expression: impl Into<String>) -> Self {
        Self::Interpolation {
            message: message.into(),
            expression: expression.into(),
            context: None,
        }
    }

    /// Create an environment variable error
    pub fn env_var(var: impl Into<String>, has_default: bool) -> Self {
        Self::EnvVar {
            var: var.into(),
            has_default,
        }
    }

    /// Create an import error
    pub fn import(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Import {
            path: path.into(),
            reason: reason.into(),
            from: None,
        }
    }

    /// Create a schema validation error
    pub fn schema(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Schema {
            path: path.into(),
            message: message.into(),
            expected: None,
        }
    }

    /// Create a circular reference error
    pub fn circular_reference(chain: impl Into<String>) -> Self {
        Self::CircularReference {
            chain: chain.into(),
        }
    }

    /// Create an internal error (should be used sparingly)
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            context: None,
        }
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            // Parse errors are generally not recoverable
            NomlError::Parse { .. } => false,
            // Validation errors might be recoverable with different input
            NomlError::Validation { .. } => true,
            // Key errors are recoverable (use defaults, etc.)
            NomlError::KeyNotFound { .. } => true,
            // Type errors might be recoverable with conversion
            NomlError::Type { .. } => true,
            // I/O errors depend on the specific error
            NomlError::Io { source, .. } => match source.kind() {
                io::ErrorKind::NotFound => true,
                io::ErrorKind::PermissionDenied => false,
                _ => true,
            },
            // Interpolation errors might be recoverable
            NomlError::Interpolation { .. } => true,
            // Environment variable errors are recoverable with defaults
            NomlError::EnvVar { has_default, .. } => *has_default,
            // Import errors might be recoverable
            NomlError::Import { .. } => true,
            // Schema errors are usually recoverable
            NomlError::Schema { .. } => true,
            // Circular references are not recoverable
            NomlError::CircularReference { .. } => false,
            // Internal errors are not recoverable
            NomlError::Internal { .. } => false,
        }
    }

    /// Get the error category for metrics/logging
    pub fn category(&self) -> &'static str {
        match self {
            NomlError::Parse { .. } => "parse",
            NomlError::Validation { .. } => "validation",
            NomlError::KeyNotFound { .. } => "key_access",
            NomlError::Type { .. } => "type_conversion",
            NomlError::Io { .. } => "io",
            NomlError::Interpolation { .. } => "interpolation",
            NomlError::EnvVar { .. } => "environment",
            NomlError::Import { .. } => "import",
            NomlError::Schema { .. } => "schema",
            NomlError::CircularReference { .. } => "circular_reference",
            NomlError::Internal { .. } => "internal",
        }
    }

    /// Get a user-friendly error message with suggestions
    pub fn user_message(&self) -> String {
        match self {
            NomlError::Parse { message, line, column, snippet } => {
                let mut msg = format!("Syntax error on line {line}, column {column}: {message}");
                if let Some(snippet) = snippet {
                    msg.push_str(&format!("\n\n{snippet}"));
                }
                msg.push_str("\n\nTip: Check for missing quotes, brackets, or commas.");
                msg
            }
            NomlError::KeyNotFound { key, available } => {
                let mut msg = format!("The key '{key}' doesn't exist.");
                if !available.is_empty() {
                    msg.push_str("\n\nDid you mean one of these?");
                    for suggestion in available {
                        msg.push_str(&format!("\n  - {suggestion}"));
                    }
                }
                msg
            }
            NomlError::EnvVar { var, has_default } => {
                let mut msg = format!("Environment variable '{var}' is not set.");
                if !has_default {
                    msg.push_str(&format!("\n\nTip: Set the environment variable or provide a default value: env(\"{var}\", \"default_value\")"));
                }
                msg
            }
            _ => self.to_string(),
        }
    }
}

/// Convert from std::io::Error to NomlError
impl From<io::Error> for NomlError {
    fn from(error: io::Error) -> Self {
        Self::Io {
            path: "<unknown>".to_string(),
            source: error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_creation_and_display() {
        let err = NomlError::parse("Invalid syntax", 10, 5);
        assert_eq!(err.to_string(), "Parse error at line 10, column 5: Invalid syntax");
    }

    #[test]
    fn error_categories() {
        let parse_err = NomlError::parse("test", 1, 1);
        assert_eq!(parse_err.category(), "parse");
        assert!(!parse_err.is_recoverable());

        let key_err = NomlError::key_not_found("test.key");
        assert_eq!(key_err.category(), "key_access");
        assert!(key_err.is_recoverable());
    }

    #[test]
    fn user_friendly_messages() {
        let err = NomlError::key_not_found_with_suggestions(
            "unknown_key",
            vec!["known_key".to_string(), "other_key".to_string()],
        );
        let msg = err.user_message();
        assert!(msg.contains("unknown_key"));
        assert!(msg.contains("known_key"));
        assert!(msg.contains("other_key"));
    }
}
