//! # NOML - Nested Object Markup Language
//! 
//! A blazing-fast, feature-rich configuration language that extends TOML
//! with advanced capabilities like variable interpolation, environment variables,
//! imports, schema validation, and native data types.
//! 
//! NOML is designed for maximum performance, safety, and developer experience
//! while maintaining perfect compatibility with existing TOML files.
//! 
//! ## Features
//! 
//! - **TOML Compatible**: All valid TOML files are valid NOML files
//! - **Variable Interpolation**: `"${section.key}"` syntax for dynamic values
//! - **Environment Variables**: `env("VAR", "default")` function calls
//! - **Native Types**: `@size("10MB")`, `@duration("30s")`, `@date("2024-01-01")`
//! - **Imports**: `include "other.noml"` for modular configurations
//! - **Comment Preservation**: Comments are maintained during parsing and serialization
//! - **Zero-Copy Parsing**: Maximum performance with minimal allocations
//! - **Rich Error Messages**: Detailed error reporting with source locations
//! - **Schema Validation**: Built-in type checking and validation
//! 
//! ## Quick Start
//! 
//! ```rust
//! use noml::{parse, Value};
//! 
//! let config = r#"
//! # Application Configuration
//! name = "MyApp"
//! version = "1.0.0"
//! debug = env("DEBUG", false)
//! 
//! [server]
//! host = "localhost"
//! port = 8080
//! timeout = @duration("30s")
//! 
//! [database]
//! url = env("DATABASE_URL")
//! max_connections = 10
//! "#;
//! 
//! // Parse the configuration
//! let document = parse(config)?;
//! let values = document.to_value()?;
//! 
//! // Access values with type safety
//! let app_name = values.get("name")?.as_string()?;
//! let server_port = values.get("server.port")?.as_integer()?;
//! let is_debug = values.get("debug")?.as_bool()?;
//! 
//! println!("App: {}, Port: {}, Debug: {}", app_name, server_port, is_debug);
//! # Ok::<(), noml::Error>(())
//! ```
//! 
//! ## Configuration Management
//! 
//! NOML provides both low-level parsing and high-level configuration management:
//! 
//! ```rust
//! use noml::Config;
//! 
//! // Load configuration from file
//! let mut config = Config::from_file("app.noml")?;
//! 
//! // Get values with defaults
//! let host = config.get("server.host", "localhost")?;
//! let port = config.get("server.port", 8080)?;
//! 
//! // Modify configuration
//! config.set("server.host", "0.0.0.0")?;
//! config.set("deployment.environment", "production")?;
//! 
//! // Save changes (preserves comments and formatting)
//! config.save()?;
//! # Ok::<(), noml::Error>(())
//! ```
//! 
//! ## Advanced Features
//! 
//! ### Variable Interpolation
//! ```toml
//! base_path = "/var/app"
//! log_path = "${base_path}/logs"
//! data_path = "${base_path}/data"
//! ```
//! 
//! ### Environment Variables
//! ```toml
//! # Required environment variable
//! api_key = env("API_KEY")
//! 
//! # With default value
//! debug_mode = env("DEBUG", false)
//! log_level = env("LOG_LEVEL", "info")
//! ```
//! 
//! ### Native Types
//! ```toml
//! # File sizes
//! max_upload_size = @size("100MB")
//! cache_limit = @size("2GB")
//! 
//! # Time durations  
//! request_timeout = @duration("30s")
//! session_lifetime = @duration("24h")
//! 
//! # Dates and times
//! created_at = @date("2024-01-01T10:00:00Z")
//! expires_at = @date("2024-12-31T23:59:59Z")
//! ```
//! 
//! ### Imports and Modularity
//! ```toml
//! # main.noml
//! name = "MyApp"
//! include "database.noml"
//! include "server.noml"
//! include "features/${environment}.noml"
//! ```

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]

// Public API exports
pub use error::{NomlError as Error, Result};
pub use parser::{parse, parse_from_file, validate, Document, AstNode};
pub use value::Value;
pub use config::{Config, ConfigBuilder};

// Core modules
pub mod error;
pub mod parser;
pub mod value;
pub mod config;

// Re-export commonly used types for convenience
pub mod prelude {
    //! Convenient imports for common NOML operations
    pub use crate::{parse, parse_from_file, validate};
    pub use crate::{Config, ConfigBuilder, Document, Value};
    pub use crate::{Error, Result};
}

/// Current version of the NOML library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Parse NOML from a string
/// 
/// This is the most common entry point for parsing NOML content.
/// It provides comprehensive error reporting and preserves all source
/// information for round-trip serialization.
/// 
/// # Example
/// ```rust
/// use noml::parse;
/// 
/// let config = r#"
/// name = "example"
/// version = 1.0
/// [database]
/// url = "sqlite://db.sqlite"
/// "#;
/// 
/// let document = parse(config)?;
/// let value = document.to_value()?;
/// 
/// assert_eq!(value.get("name").unwrap().as_string().unwrap(), "example");
/// # Ok::<(), noml::Error>(())
/// ```
pub fn parse(source: &str) -> Result<Document> {
    parser::parse(source)
}

/// Parse NOML from a file
/// 
/// Convenience function for parsing NOML files with proper error
/// reporting that includes the file path.
/// 
/// # Example
/// ```rust,no_run
/// use noml::parse_from_file;
/// 
/// let document = parse_from_file("config.noml")?;
/// let value = document.to_value()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Document> {
    parser::parse_from_file(path)
}

/// Validate NOML syntax without full parsing
/// 
/// This is faster than full parsing when you only need to check
/// syntax validity.
/// 
/// # Example
/// ```rust
/// use noml::validate;
/// 
/// let valid_config = r#"
/// name = "test"
/// [section]
/// key = "value"
/// "#;
/// 
/// assert!(validate(valid_config).is_ok());
/// 
/// let invalid_config = r#"
/// name = "test"
/// [section
/// key = "value"
/// "#;
/// 
/// assert!(validate(invalid_config).is_err());
/// ```
pub fn validate(source: &str) -> Result<()> {
    parser::validate(source)
}

/// Convert a Rust value to NOML string representation
/// 
/// This function serializes any value that implements `Into<Value>`
/// into a NOML string format.
/// 
/// # Example
/// ```rust
/// use noml::{to_string, Value};
/// use std::collections::BTreeMap;
/// 
/// let mut config = BTreeMap::new();
/// config.insert("name".to_string(), Value::string("MyApp"));
/// config.insert("version".to_string(), Value::string("1.0.0"));
/// config.insert("debug".to_string(), Value::bool(true));
/// 
/// let noml_string = to_string(&Value::table(config))?;
/// println!("{}", noml_string);
/// # Ok::<(), noml::Error>(())
/// ```
pub fn to_string(value: &Value) -> Result<String> {
    // TODO: Implement serialization in future iteration
    // For now, we focus on parsing capabilities
    todo!("Serialization will be implemented in the next iteration")
}

/// Convert a Rust value to a pretty-printed NOML string
/// 
/// Similar to `to_string` but with formatting optimized for readability.
/// 
/// # Example
/// ```rust,no_run
/// use noml::{to_string_pretty, Value};
/// 
/// let value = Value::string("Hello, NOML!");
/// let pretty = to_string_pretty(&value)?;
/// # Ok::<(), noml::Error>(())
/// ```
pub fn to_string_pretty(value: &Value) -> Result<String> {
    // TODO: Implement pretty serialization in future iteration
    todo!("Pretty serialization will be implemented in the next iteration")
}

/// Library information and capabilities
pub mod info {
    //! Information about the NOML library and its capabilities
    
    /// Features supported by this build of NOML
    pub struct Features {
        /// Support for date/time types via chrono
        pub chrono: bool,
        /// Support for serde serialization
        pub serde: bool,
    }
    
    /// Get information about supported features
    pub fn features() -> Features {
        Features {
            chrono: cfg!(feature = "chrono"),
            serde: true, // Always available
        }
    }
    
    /// Get the library version
    pub fn version() -> &'static str {
        crate::VERSION
    }
    
    /// Get supported NOML specification version
    pub fn spec_version() -> &'static str {
        "1.0.0"
    }
    
    /// Check if this build supports a specific feature
    pub fn has_feature(feature: &str) -> bool {
        match feature {
            "chrono" => cfg!(feature = "chrono"),
            "serde" => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_info() {
        assert!(!VERSION.is_empty());
        assert_eq!(info::version(), VERSION);
        assert!(!info::spec_version().is_empty());
    }

    #[test]
    fn feature_detection() {
        let features = info::features();
        
        // serde should always be available
        assert!(features.serde);
        assert!(info::has_feature("serde"));
        
        // chrono availability depends on feature flags
        assert_eq!(features.chrono, cfg!(feature = "chrono"));
        assert_eq!(info::has_feature("chrono"), cfg!(feature = "chrono"));
        
        // Non-existent feature
        assert!(!info::has_feature("non_existent"));
    }

    #[test]
    fn basic_parsing_integration() {
        let config = r#"
        # Basic configuration test
        name = "integration_test"
        version = 1.0
        enabled = true
        
        [section]
        key = "value"
        number = 42
        
        [nested.deeply]
        value = "deep"
        "#;
        
        let document = parse(config).expect("Should parse successfully");
        let values = document.to_value().expect("Should convert to values");
        
        // Test basic value access
        assert_eq!(values.get("name").unwrap().as_string().unwrap(), "integration_test");
        assert_eq!(values.get("version").unwrap().as_float().unwrap(), 1.0);
        assert_eq!(values.get("enabled").unwrap().as_bool().unwrap(), true);
        
        // Test nested access
        assert_eq!(values.get("section.key").unwrap().as_string().unwrap(), "value");
        assert_eq!(values.get("section.number").unwrap().as_integer().unwrap(), 42);
        assert_eq!(values.get("nested.deeply.value").unwrap().as_string().unwrap(), "deep");
    }

    #[test]
    fn validation_integration() {
        let valid = r#"
        name = "test"
        [section]
        key = "value"
        "#;
        
        assert!(validate(valid).is_ok());
        
        let invalid = r#"
        name = "test"
        [section
        key = "value"
        "#;
        
        assert!(validate(invalid).is_err());
    }

    #[test]
    fn error_handling() {
        let invalid_noml = r#"
        name = "test"
        version = 1.0.0.0.0  # Invalid float
        "#;
        
        match parse(invalid_noml) {
            Ok(_) => panic!("Should have failed to parse"),
            Err(error) => {
                assert_eq!(error.category(), "parse");
                assert!(!error.to_string().is_empty());
            }
        }
    }

    #[test]
    fn file_parsing_simulation() {
        // Since we can't create actual files in tests easily,
        // we'll test the parse function directly
        let config = r#"
        # File-based configuration
        app_name = "FileApp"
        
        [logging]
        level = "info"
        file = "/var/log/app.log"
        "#;
        
        let document = parse(config).expect("Should parse file content");
        let values = document.to_value().expect("Should convert to values");
        
        assert_eq!(values.get("app_name").unwrap().as_string().unwrap(), "FileApp");
        assert_eq!(values.get("logging.level").unwrap().as_string().unwrap(), "info");
        assert_eq!(values.get("logging.file").unwrap().as_string().unwrap(), "/var/log/app.log");
    }

    #[test]
    fn comprehensive_feature_test() {
        let config = r#"
        # Comprehensive NOML feature test
        name = "comprehensive_test"
        
        # Basic types
        string_val = "hello world"
        integer_val = 42
        float_val = 3.14159
        bool_val = true
        null_val = null
        
        # Arrays
        simple_array = [1, 2, 3]
        mixed_array = ["string", 42, true]
        
        # Nested structures
        [database]
        host = "localhost"
        port = 5432
        
        [database.pool]
        min_connections = 5
        max_connections = 20
        
        # Inline tables
        server = { host = "0.0.0.0", port = 8080 }
        
        # Complex nesting
        [services.cache.redis]
        host = "redis.example.com"
        port = 6379
        "#;
        
        let document = parse(config).expect("Should parse comprehensive config");
        let values = document.to_value().expect("Should convert to values");
        
        // Test all basic types
        assert_eq!(values.get("name").unwrap().as_string().unwrap(), "comprehensive_test");
        assert_eq!(values.get("string_val").unwrap().as_string().unwrap(), "hello world");
        assert_eq!(values.get("integer_val").unwrap().as_integer().unwrap(), 42);
        assert!((values.get("float_val").unwrap().as_float().unwrap() - 3.14159).abs() < f64::EPSILON);
        assert_eq!(values.get("bool_val").unwrap().as_bool().unwrap(), true);
        assert!(values.get("null_val").unwrap().is_null());
        
        // Test arrays
        let simple_array = values.get("simple_array").unwrap().as_array().unwrap();
        assert_eq!(simple_array.len(), 3);
        assert_eq!(simple_array[0].as_integer().unwrap(), 1);
        
        let mixed_array = values.get("mixed_array").unwrap().as_array().unwrap();
        assert_eq!(mixed_array.len(), 3);
        assert_eq!(mixed_array[0].as_string().unwrap(), "string");
        assert_eq!(mixed_array[1].as_integer().unwrap(), 42);
        assert_eq!(mixed_array[2].as_bool().unwrap(), true);
        
        // Test nested structures
        assert_eq!(values.get("database.host").unwrap().as_string().unwrap(), "localhost");
        assert_eq!(values.get("database.port").unwrap().as_integer().unwrap(), 5432);
        assert_eq!(values.get("database.pool.min_connections").unwrap().as_integer().unwrap(), 5);
        assert_eq!(values.get("database.pool.max_connections").unwrap().as_integer().unwrap(), 20);
        
        // Test inline tables
        assert_eq!(values.get("server.host").unwrap().as_string().unwrap(), "0.0.0.0");
        assert_eq!(values.get("server.port").unwrap().as_integer().unwrap(), 8080);
        
        // Test complex nesting
        assert_eq!(values.get("services.cache.redis.host").unwrap().as_string().unwrap(), "redis.example.com");
        assert_eq!(values.get("services.cache.redis.port").unwrap().as_integer().unwrap(), 6379);
    }

    #[test]
    fn comment_preservation_test() {
        let config = r#"
        # Main application config
        name = "CommentTest" # App name
        
        # Database configuration section
        [database]
        # Connection settings
        host = "localhost" # Default host
        port = 5432
        "#;
        
        let document = parse(config).expect("Should parse config with comments");
        let comments = document.all_comments();
        
        // Should have preserved multiple comments
        assert!(!comments.is_empty());
        
        // Check for specific comment content
        let comment_texts: Vec<&str> = comments.iter().map(|c| c.text.as_str()).collect();
        assert!(comment_texts.iter().any(|&text| text.contains("Main application config")));
        assert!(comment_texts.iter().any(|&text| text.contains("App name")));
        assert!(comment_texts.iter().any(|&text| text.contains("Database configuration section")));
        assert!(comment_texts.iter().any(|&text| text.contains("Connection settings")));
        assert!(comment_texts.iter().any(|&text| text.contains("Default host")));
    }
}
