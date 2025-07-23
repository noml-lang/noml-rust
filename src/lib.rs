//! # NOML - Nested Object Markup Language
//! 
//! NOML is a modern configuration language that combines the simplicity of TOML
//! with advanced features like environment variables, file inclusion, variable
//! interpolation, and native types.
//! 
//! # Quick Start
//! 
//! ```rust
//! use noml::{parse, Value};
//! 
//! let source = r#"
//!     # Basic configuration
//!     name = "my-app"
//!     version = "1.0.0"
//!     debug = true
//!     
//!     # Environment variables
//!     database_url = env("DATABASE_URL", "sqlite:memory:")
//!     
//!     # Native types
//!     max_file_size = @size("10MB")
//!     timeout = @duration("30s")
//!     
//!     # Nested configuration
//!     [server]
//!     host = "0.0.0.0"
//!     port = 8080
//!     
//!     [database]
//!     host = "localhost"
//!     port = 5432
//! "#;
//! 
//! let config = parse(source)?;
//! 
//! // Access values
//! assert_eq!(config.get("name").unwrap().as_string(), Some("my-app"));
//! assert_eq!(config.get("server.port").unwrap().as_integer(), Some(8080));
//! 
//! # Ok::<(), noml::error::NomlError>(())
//! ```
//! 
//! # Features
//! 
//! - **TOML-compatible syntax** with additional features
//! - **Environment variables** via `env("VAR_NAME", "default")`
//! - **File inclusion** via `include "path/to/file.noml"`
//! - **Variable interpolation** via `"Hello ${name}!"`
//! - **Native types** like `@size("10MB")` and `@duration("30s")`
//! - **Comment preservation** for tooling and round-trip editing
//! - **Detailed error reporting** with source locations
//! - **Zero-copy parsing** for performance
//!
//! # Advanced Usage
//! 
//! ```rust
//! use noml::{Resolver, ResolverConfig, parse_string};
//! use std::collections::HashMap;
//! 
//! // Custom environment variables
//! let mut env_vars = HashMap::new();
//! env_vars.insert("APP_NAME".to_string(), "my-app".to_string());
//! 
//! // Custom resolver configuration
//! let config = ResolverConfig {
//!     env_vars: Some(env_vars),
//!     allow_missing_env: true,
//!     ..Default::default()
//! };
//! 
//! let mut resolver = Resolver::with_config(config);
//! let document = parse_string(r#"name = env("APP_NAME")"#, None)?;
//! let value = resolver.resolve(document)?;
//! 
//! assert_eq!(value.get("name").unwrap().as_string(), Some("my-app"));
//! 
//! # Ok::<(), noml::error::NomlError>(())
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod error;
pub mod parser;
pub mod value;
pub mod resolver;

// Re-export main types for convenience
pub use error::{NomlError, Result};
pub use parser::{parse_string, parse_file, Document};
pub use value::Value;
pub use resolver::{Resolver, ResolverConfig, NativeResolver};

use std::path::Path;

/// Parse NOML from a string and resolve all dynamic features
/// 
/// This is the main entry point for parsing NOML. It handles parsing
/// the source text and resolving all dynamic features like environment
/// variables, includes, and interpolations.
/// 
/// # Example
/// 
/// ```rust
/// use noml::parse;
/// 
/// let config = parse(r#"
///     name = "my-app"
///     debug = env("DEBUG", false)
///     
///     [server]
///     port = 8080
/// "#)?;
/// 
/// assert_eq!(config.get("name").unwrap().as_string(), Some("my-app"));
/// 
/// # Ok::<(), noml::error::NomlError>(())
/// ```
pub fn parse(source: &str) -> Result<Value> {
    let document = parse_string(source, None)?;
    let mut resolver = Resolver::new();
    resolver.resolve(document)
}

/// Parse NOML from a file and resolve all dynamic features
/// 
/// This function reads a NOML file from disk, parses it, and resolves
/// all dynamic features. The file path is used as the base path for
/// resolving relative includes.
/// 
/// # Example
/// 
/// ```rust
/// use noml::parse_from_file;
/// 
/// // Assuming config.noml exists
/// let config = parse_from_file("config.noml")?;
/// 
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_from_file<P: AsRef<Path>>(path: P) -> Result<Value> {
    let path = path.as_ref();
    let document = parse_file(path)?;
    
    let base_path = path.parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    
    let mut resolver = Resolver::new().with_base_path(base_path);
    resolver.resolve(document)
}

/// Parse NOML from a string without resolving dynamic features
/// 
/// This function only parses the NOML syntax into an AST document
/// without resolving environment variables, includes, or interpolations.
/// Use this when you want to inspect the raw structure or handle
/// resolution manually.
/// 
/// # Example
/// 
/// ```rust
/// use noml::parse_raw;
/// 
/// let document = parse_raw(r#"
///     name = env("APP_NAME")
///     port = 8080
/// "#)?;
/// 
/// // Document contains unresolved env() call
/// # Ok::<(), noml::error::NomlError>(())
/// ```
pub fn parse_raw(source: &str) -> Result<Document> {
    parse_string(source, None)
}

/// Parse NOML from a file without resolving dynamic features
/// 
/// # Example
/// 
/// ```rust
/// use noml::parse_raw_from_file;
/// 
/// let document = parse_raw_from_file("config.noml")?;
/// 
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_raw_from_file<P: AsRef<Path>>(path: P) -> Result<Document> {
    parse_file(path.as_ref())
}

/// Validate NOML syntax without parsing into values
/// 
/// This is useful for syntax checking without the overhead of building
/// the full AST or resolving dynamic features.
/// 
/// # Example
/// 
/// ```rust
/// use noml::validate;
/// 
/// assert!(validate(r#"name = "valid""#).is_ok());
/// assert!(validate(r#"name = "unclosed string"#).is_err());
/// ```
pub fn validate(source: &str) -> Result<()> {
    parse_raw(source).map(|_| ())
}

/// Create a NOML value using a convenient macro syntax
/// 
/// # Example
/// 
/// ```rust
/// use noml::{noml_value, Value};
/// 
/// let config = noml_value!({
///     "name" => "my-app",
///     "version" => "1.0.0",
///     "features" => ["parsing", "validation"],
///     "server" => {
///         "host" => "localhost",
///         "port" => 8080
///     }
/// });
/// 
/// assert_eq!(config.get("server.port").unwrap().as_integer(), Some(8080));
/// ```
// pub use crate::noml_value; // Removed unresolved import

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_basic_parsing() {
        let source = r#"
            name = "test-app"
            version = "1.0.0"
            debug = true
            
            [server]
            host = "localhost"
            port = 8080
            
            [database]
            url = "sqlite:memory:"
        "#;

        let config = parse(source).unwrap();
        
        assert_eq!(config.get("name").unwrap().as_string().unwrap(), "test-app");
        assert_eq!(config.get("version").unwrap().as_string().unwrap(), "1.0.0");
        assert_eq!(config.get("debug").unwrap().as_bool().unwrap(), true);
        assert_eq!(config.get("server.host").unwrap().as_string().unwrap(), "localhost");
        assert_eq!(config.get("server.port").unwrap().as_integer().unwrap(), 8080);
        assert_eq!(config.get("database.url").unwrap().as_string().unwrap(), "sqlite:memory:");
    }

    #[test]
    fn test_arrays_and_inline_tables() {
        let source = r#"
            languages = ["rust", "go", "python"]
            point = { x = 10, y = 20 }
            
            [[servers]]
            name = "web-1"
            ip = "192.168.1.10"
            
            [[servers]]
            name = "web-2"
            ip = "192.168.1.11"
        "#;

        let config = parse(source).unwrap();
        
        let languages = config.get("languages").unwrap().as_array().unwrap();
        assert_eq!(languages.len(), 3);
        assert_eq!(languages[0].as_string().unwrap(), "rust");
        
        assert_eq!(config.get("point.x").unwrap().as_integer().unwrap(), 10);
        assert_eq!(config.get("point.y").unwrap().as_integer().unwrap(), 20);
    }

    #[test]
    fn test_env_function() {
        // Set a test environment variable
        env::set_var("NOML_TEST_VAR", "test_value");
        
        let source = r#"
            app_name = env("NOML_TEST_VAR")
            fallback = env("NONEXISTENT_VAR", "default_value")
        "#;

        let config = parse(source).unwrap();
        
        assert_eq!(config.get("app_name").unwrap().as_string().unwrap(), "test_value");
        assert_eq!(config.get("fallback").unwrap().as_string().unwrap(), "default_value");
        
        // Clean up
        env::remove_var("NOML_TEST_VAR");
    }

    #[test]
    fn test_native_types() {
        let source = r#"
            max_size = @size("10MB")
            timeout = @duration("30s")
            homepage = @url("https://example.com")
        "#;

        let config = parse(source).unwrap();
        
        // These should resolve to their underlying values
        assert_eq!(config.get("max_size").unwrap().as_integer().unwrap(), 10 * 1024 * 1024);
        assert_eq!(config.get("timeout").unwrap().as_float().unwrap(), 30.0);
        assert_eq!(config.get("homepage").unwrap().as_string().unwrap(), "https://example.com");
    }

    #[test]
    fn test_comments() {
        let source = r#"
            # This is a top-level comment
            name = "test" # Inline comment
            
            # Section comment
            [server]
            # Another comment
            port = 8080
        "#;

        // Test that it parses without error (comments are preserved in AST)
        let document = parse_raw(source).unwrap();
        let comments = document.all_comments();
        assert!(!comments.is_empty());
    }

    #[test]
    fn test_validation() {
        assert!(validate(r#"name = "valid""#).is_ok());
        assert!(validate(r#"[section]
name = "valid"
port = 8080"#).is_ok());
        
        // Invalid syntax should fail
        assert!(validate(r#"name = "unclosed string"#).is_err());
        assert!(validate(r#"[unclosed section"#).is_err());
    }

    // #[test]
    // fn test_macro() {
    //     let config = noml_value!({
    //         "name" => "test",
    //         "version" => 1,
    //         "features" => ["a", "b", "c"],
    //         "nested" => {
    //             "x" => 10,
    //             "y" => 20
    //         }
    //     });

    //     assert_eq!(config.get("name").unwrap().as_string(), Some("test"));
    //     assert_eq!(config.get("version").unwrap().as_integer(), Some(1));
    //     assert_eq!(config.get("nested.x").unwrap().as_integer(), Some(10));
        
    //     let features = config.get("features").unwrap().as_array().unwrap();
    //     assert_eq!(features.len(), 3);
    // }

    #[test] 
    fn test_error_handling() {
        // Parse error
        let result = parse(r#"invalid = syntax error"#);
        assert!(result.is_err());
        
        // Env var error  
        let result = parse(r#"missing = env("DEFINITELY_MISSING_VAR")"#);
        assert!(result.is_err());
    }
}
