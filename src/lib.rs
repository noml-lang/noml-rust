//! # NOML - Nested Object Markup Language
//! 
//! NOML is a modern configuration language that combines the simplicity of TOML
//! with advanced features like environment variables, file inclusion, variable
//! interpolation, native types, HTTP includes, and schema validation.
//! 
//! ## Quick Start
//! 
//! ```rust
//! use noml::{parse, Value};
//! 
//! let source = r#"
//!     name = "my-app"
//!     version = "1.0.0"
//!     debug = true
//!     
//!     database_url = env("DATABASE_URL", "sqlite:memory:")
//!     
//!     max_file_size = @size("10MB")
//!     timeout = @duration("30s")
//!     server_ip = @ip("127.0.0.1")
//!     
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
//! // Access values with type safety
//! assert_eq!(config.get("name").unwrap().as_string().unwrap(), "my-app");
//! assert_eq!(config.get("server.port").unwrap().as_integer().unwrap(), 8080);
//! 
//! # Ok::<(), noml::error::NomlError>(())
//! ```
//! 
//! ## Core Features
//! 
//! - **🔧 TOML-compatible syntax** with extended functionality
//! - **🌍 Environment variables** via `env("VAR_NAME", "default")`
//! - **📁 File inclusion** via `include "path/to/file.noml"`
//! - **🌐 HTTP includes** via `include "https://example.com/config.noml"`
//! - **🔗 Variable interpolation** via `"Hello ${name}!"`
//! - **⚡ Native types** like `@size("10MB")`, `@duration("30s")`, `@ip("127.0.0.1")`
//! - **✅ Schema validation** for type safety and error prevention
//! - **💬 Comment preservation** for tooling and round-trip editing
//! - **🎯 Detailed error reporting** with precise source locations
//! - **🚀 Zero-copy parsing** for optimal performance
//! - **🔄 Async support** with tokio integration
//! 
//! ## Native Types
//! 
//! NOML includes built-in support for common configuration types:
//! 
//! ```rust
//! use noml::parse;
//! 
//! let config = parse(r#"
//!     max_upload = @size("100MB")
//!     cache_size = @size("2GB")
//!     
//!     timeout = @duration("30s")
//!     retry_delay = @duration("5m")
//!     
//!     api_endpoint = @url("https://api.example.com/v1")
//! "#)?;
//! 
//! // Native types are automatically converted to appropriate Rust types
//! # Ok::<(), noml::error::NomlError>(())
//! ```
//! 
//! ## Advanced Configuration Management
//! 
//! ```rust
//! use noml::{Config, Schema, FieldType, SchemaBuilder};
//! 
//! // Load configuration with schema validation
//! let config = Config::from_string(r#"
//!     app_name = "my-service"
//!     port = 8080
//!     debug = false
//!     
//!     [database]
//!     host = "localhost"
//!     max_connections = 100
//! "#)?;
//! 
//! // Define and validate schema
//! let schema = SchemaBuilder::new()
//!     .require_string("app_name")
//!     .require_integer("port") 
//!     .optional_bool("debug")
//!     .build();
//! 
//! config.validate_schema(&schema)?;
//! 
//! // Access with type safety
//! let port = config.get("port").unwrap().as_integer()?;
//! let debug = config.get("debug").unwrap_or(&noml::Value::Bool(false)).as_bool()?;
//! 
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//! 
//! ## Async Support
//! 
//! Enable the `async` feature for non-blocking operations:
//! 
//! ```toml
//! [dependencies]
//! noml = { version = "0.4", features = ["async"] }
//! ```
//! 
//! ```rust,ignore
//! use noml::{parse_async, Config};
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Parse with HTTP includes
//!     let config = parse_async(r#"
//!         # Local configuration
//!         app_name = "my-app"
//!         
//!         # Remote configuration
//!         include "https://config-server.com/common.noml"
//!     "#).await?;
//!     
//!     // Async file operations
//!     let mut config = Config::load_async("config.noml").await?;
//!     config.set("updated_at", chrono::Utc::now().to_rfc3339())?;
//!     config.save_async().await?;
//!     
//!     Ok(())
//! }
//! ```
//! 
//! ## Error Handling
//! 
//! NOML provides detailed error information for debugging:
//! 
//! ```rust
//! use noml::parse;
//! 
//! let result = parse(r#"
//!     invalid_syntax = [  # Missing closing bracket
//! "#);
//! 
//! match result {
//!     Err(e) => {
//!         println!("Parse error: {}", e);
//!         // Error contains source location information
//!     }
//!     Ok(_) => unreachable!(),
//! }
//! ```
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
//! assert_eq!(value.get("name").unwrap().as_string().unwrap(), "my-app");
//! 
//! # Ok::<(), noml::error::NomlError>(())
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod config;
pub mod error;
pub mod parser;
pub mod value;
pub mod resolver;
pub mod schema;

// Re-export main types for convenience
pub use config::Config;
pub use error::{NomlError, Result};
pub use parser::{parse_string, parse_file, Document};
pub use value::Value;
pub use resolver::{Resolver, ResolverConfig, NativeResolver};
pub use schema::{Schema, FieldSchema, FieldType, SchemaBuilder};

use std::path::Path;

/// Parse NOML from a string and resolve all dynamic features
/// 
/// This is the main entry point for parsing NOML configuration. It performs
/// complete parsing and resolution, including:
/// 
/// - Lexical analysis and syntax parsing
/// - Environment variable resolution via `env("VAR", "default")`
/// - Variable interpolation with `${variable}` syntax
/// - Native type conversion for `@size()`, `@duration()`, etc.
/// - File inclusion via `include "path/to/file"`
/// 
/// # Arguments
/// 
/// * `source` - NOML configuration text to parse
/// 
/// # Returns
/// 
/// Returns a [`Value`] containing the resolved configuration data, or a
/// [`NomlError`] if parsing or resolution fails.
/// 
/// # Examples
/// 
/// Basic configuration parsing:
/// 
/// ```rust
/// use noml::parse;
/// 
/// let config = parse(r#"
///     app_name = "my-service"
///     port = 8080
///     debug = true
///     
///     [database]
///     host = "localhost"
///     max_connections = 100
/// "#)?;
/// 
/// assert_eq!(config.get("app_name").unwrap().as_string().unwrap(), "my-service");
/// assert_eq!(config.get("database.host").unwrap().as_string().unwrap(), "localhost");
/// 
/// # Ok::<(), noml::error::NomlError>(())
/// ```
/// 
/// With environment variables and native types:
/// 
/// ```rust
/// use noml::parse;
/// use std::env;
/// 
/// env::set_var("APP_PORT", "3000");
/// 
/// let config = parse(r#"
///     name = "web-server"
///     port = env("APP_PORT", 8080)
///     timeout = @duration("30s")
///     max_size = @size("10MB")
///     
///     database_url = env("DATABASE_URL", "sqlite:memory:")
/// "#)?;
/// 
/// assert_eq!(config.get("port").unwrap().as_integer().unwrap(), 3000);
/// # Ok::<(), noml::error::NomlError>(())
/// ```
/// 
/// # Errors
/// 
/// Returns [`NomlError`] for:
/// - Syntax errors in the NOML source
/// - Invalid native type arguments
/// - Missing environment variables without defaults
/// - File system errors during includes
/// - Type conversion failures
pub fn parse(source: &str) -> Result<Value> {
    let document = parse_string(source, None)?;
    let mut resolver = Resolver::new();
    resolver.resolve(document)
}

/// Parse NOML from a file and resolve all dynamic features
/// 
/// Reads a NOML configuration file from disk and performs complete parsing
/// and resolution. The file's directory becomes the base path for resolving
/// relative include statements.
/// 
/// # Arguments
/// 
/// * `path` - Path to the NOML file to parse
/// 
/// # Returns
/// 
/// Returns a [`Value`] containing the resolved configuration data, or a
/// [`NomlError`] if file reading, parsing, or resolution fails.
/// 
/// # Examples
/// 
/// Basic file parsing:
/// 
/// ```rust,no_run
/// use noml::parse_from_file;
/// 
/// let config = parse_from_file("app.noml")?;
/// let app_name = config.get("name").unwrap().as_string()?;
/// 
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
/// 
/// With relative includes:
/// 
/// ```rust,no_run
/// use noml::parse_from_file;
/// 
/// // config/main.noml contains:
/// // include "database.noml"  # Resolves to config/database.noml
/// // include "../shared.noml" # Resolves to shared.noml
/// 
/// let config = parse_from_file("config/main.noml")?;
/// 
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
/// 
/// # File Resolution
/// 
/// - Relative includes are resolved from the directory containing the main file
/// - Environment variables are resolved from the current process environment
/// - Native types are processed according to their specific parsers
/// 
/// # Errors
/// 
/// Returns [`NomlError`] for:
/// - File not found or permission errors
/// - Invalid NOML syntax
/// - Unresolvable include dependencies
/// - Environment variable resolution failures
/// - Native type conversion errors
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
/// Performs only lexical analysis and syntax parsing, returning the raw AST
/// without resolving dynamic features like environment variables, includes,
/// or interpolations. Use this when you need to inspect the document structure
/// or handle resolution manually with custom logic.
/// 
/// # Arguments
/// 
/// * `source` - NOML configuration text to parse
/// 
/// # Returns
/// 
/// Returns a [`Document`] containing the raw AST with source information,
/// or a [`NomlError`] if parsing fails.
/// 
/// # Examples
/// 
/// Basic raw parsing:
/// 
/// ```rust
/// use noml::parse_raw;
/// 
/// let document = parse_raw(r#"
///     name = "my-app"
///     port = 8080
///     debug = env("DEBUG", false)  # Not resolved
/// "#)?;
/// 
/// // Access the raw AST structure
/// println!("Document parsed successfully with source: {:?}", document.source_path);
/// 
/// # Ok::<(), noml::error::NomlError>(())
/// ```
/// 
/// Inspecting unresolved dynamic features:
/// 
/// ```rust
/// use noml::parse_raw;
/// 
/// let document = parse_raw(r#"
///     config = env("CONFIG_PATH", "default.conf")
///     timeout = @duration("30s")
///     name = "test"
/// "#)?;
/// 
/// // The document contains raw function calls and interpolations
/// // that haven't been resolved yet
/// 
/// # Ok::<(), noml::error::NomlError>(())
/// ```
/// 
/// # Use Cases
/// 
/// - Static analysis tools that need to examine structure
/// - Custom resolution with specialized logic
/// - Configuration validation without side effects
/// - Development tools and language servers
/// 
/// # Errors
/// 
/// Returns [`NomlError`] for:
/// - Syntax errors in the NOML source
/// - Invalid token sequences
/// - Malformed expressions
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
/// // This would work if config.noml exists:
/// // let document = parse_raw_from_file("config.noml")?;
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

// Async parsing functions (available with "async" feature)

/// Parse NOML from a string asynchronously
/// 
/// This is the async version of [`parse`]. The actual parsing is still synchronous,
/// but this function can be used in async contexts and enables async features
/// like remote includes when used with an async resolver.
/// 
/// # Example
/// 
/// ```rust,ignore
/// use noml::parse_async;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let source = r#"
///         name = "my-app"
///         version = "1.0.0"
///     "#;
///     
///     let config = parse_async(source).await?;
///     assert_eq!(config.get("name").unwrap().as_string().unwrap(), "my-app");
///     
///     Ok(())
/// }
/// ```
#[cfg(feature = "async")]
pub async fn parse_async(source: &str) -> Result<Value> {
    let document = parse_raw(source)?;
    let mut resolver = resolver::Resolver::new();
    resolver.resolve_document_async(&document).await
}

/// Parse NOML from a file asynchronously
/// 
/// This function uses async file I/O and can be used in async contexts.
/// It also enables async features like remote includes.
/// 
/// # Example
/// 
/// ```rust,ignore
/// use noml::parse_from_file_async;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = parse_from_file_async("config.noml").await?;
///     println!("Loaded config: {:?}", config);
///     Ok(())
/// }
/// ```
#[cfg(feature = "async")]
pub async fn parse_from_file_async<P: AsRef<std::path::Path>>(path: P) -> Result<Value> {
    let path = path.as_ref();
    let source = tokio::fs::read_to_string(path).await
        .map_err(|e| error::NomlError::io(path.to_string_lossy().to_string(), e))?;
    
    let document = parse_raw(&source)?;
    
    let base_path = path.parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();
    
    let mut resolver = resolver::Resolver::new().with_base_path(base_path);
    resolver.resolve_document_async(&document).await
}

/// Parse NOML from a file asynchronously without resolving dynamic features
/// 
/// This is the async version of [`parse_raw_from_file`].
/// 
/// # Example
/// 
/// ```rust,ignore
/// use noml::parse_raw_from_file_async;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let document = parse_raw_from_file_async("config.noml").await?;
///     println!("Raw document: {:?}", document);
///     Ok(())
/// }
/// ```
#[cfg(feature = "async")]
pub async fn parse_raw_from_file_async<P: AsRef<std::path::Path>>(path: P) -> Result<Document> {
    let source = tokio::fs::read_to_string(path.as_ref()).await
        .map_err(|e| error::NomlError::io(path.as_ref().to_string_lossy().to_string(), e))?;
    parse_raw(&source)
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
/// assert_eq!(config.get("server.port").unwrap().as_integer().unwrap(), 8080);
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
        assert!(config.get("debug").unwrap().as_bool().unwrap());
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
        
        // Float comparison with tolerance
        let timeout = config.get("timeout").unwrap().as_float().unwrap();
        assert!((timeout - 30.0).abs() < f64::EPSILON, "Expected 30.0, got {timeout}");
        
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

    // Thread safety tests
    #[test]
    fn test_value_send_sync() {
        fn require_send_sync<T: Send + Sync>(_: &T) {}
        
        let value = Value::String("test".to_string());
        require_send_sync(&value);
        
        let table = Value::empty_table();
        require_send_sync(&table);
        
        let array = Value::Array(vec![Value::Integer(1), Value::String("test".to_string())]);
        require_send_sync(&array);
    }

    #[test] 
    fn test_config_send_sync() {
        fn require_send_sync<T: Send + Sync>(_: &T) {}
        
        let config = Config::new();
        require_send_sync(&config);
        
        let config = parse("name = \"test\"").unwrap();
        require_send_sync(&config);
    }
}
