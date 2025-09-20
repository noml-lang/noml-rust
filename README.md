<div align="center">
    <h1>
        <span>NOML +&nbsp; RUST</span>
        <br>
        <sub><sup>PARSER &amp; GENERATOR</sup></sub>
    </h1>
</div>

<div align="center">
    <div>
        <a href="https://crates.io/crates/noml" alt="NOML on Crates.io"><img alt="Crates.io" src="https://img.shields.io/crates/v/noml"></a>
        <span>&nbsp;</span>
        <a href="https://crates.io/crates/noml" alt="Download NOML"><img alt="Crates.io Downloads" src="https://img.shields.io/crates/d/noml?color=%230099ff"></a>
        <span>&nbsp;</span>
        <a href="https://docs.rs/noml" title="NOML Documentation"><img alt="docs.rs" src="https://img.shields.io/docsrs/noml"></a>
        <span>&nbsp;</span>
        <img alt="GitHub last commit" src="https://img.shields.io/github/last-commit/noml-lang/noml-rust?membase=%23347d39" alt="last commit badge">
    </div>
</div>
<br><br>
<p><strong>Nested-Object Markup Language</strong> (<strong>NOML</strong>) is a powerful, modern configuration language designed for clarity, ease of use, and a rich feature set. This crate provides a blazing-fast and full-fidelity parser and generator for <code>noml</code> in Rust.</p>
<p><strong>NOML</strong> combines the simplicity of <abbr title="Tom's Obvious, Minimal Language"><b>TOML</b></abbr> with advanced, developer-friendly features, making it an ideal choice for a wide range of applications, from simple configuration files to complex, dynamic settings.</p>

<br>


<h2 align="center">
    ⚠️<br>
    PRE-RELEASE<br>
    <sup><sub>PROJECT IN-DEVELOPMENT</sub></sup>
    <br><br>
</h2>
<br>

<h2>Key Features:</h2>
<ul>
    <li>
        <b>Intuitive, TOML-like Syntax:</b> &nbsp; Easy to read and write, with a familiar structure.
    </li>
    <li>
        <b>Environment Variable Interpolation:</b> &nbsp; Seamlessly pull in configuration from the environment with <code>env("VAR_NAME", "default_value")</code>.
    </li>
    <li>
        <b>File Imports:</b> &nbsp; Organize your configuration into multiple files with <code>include "path/to/file.noml"</code>.
    </li>
    <li>
        <b>HTTP Includes:</b> &nbsp; Fetch remote configuration with <code>include "https://example.com/config.noml"</code> using the async resolver.
    </li>
    <li>
        <b>Variable Interpolation:</b> &nbsp; Reference other values in your configuration with <code>${path.to.variable}</code>.
    </li>
    <li>
        <b>Native Types:</b> &nbsp; Go beyond simple primitives with built-in types like <code>@size("10MB")</code>, <code>@duration("30s")</code>, <code>@url("https://example.com")</code>, <code>@ip("192.168.1.1")</code>, <code>@semver("1.2.3")</code>, <code>@base64("SGVsbG8=")</code>, and <code>@uuid("550e8400-e29b-41d4-a716-446655440000")</code>.
    </li>
    <li>
        <b>Full Fidelity Parsing:</b> &nbsp; The parser preserves all comments, whitespace, and formatting, allowing you to programmatically edit and save NOML files without losing any information.
    </li>
    <li>
        <b>Blazing Fast:</b> &nbsp; Built with performance in mind, featuring a zero-copy lexer and an efficient, hand-written parser.
    </li>
    <li>
        <b>Excellent Error Reporting:</b> &nbsp; Get clear, detailed error messages with precise source locations to quickly debug your configuration files.
    </li>
    <li>
        <b>High-Level Config Management:</b> &nbsp; A simple and powerful API for loading, modifying, and saving configurations.
    </li>
    <li>
        <b>Async Support:</b> &nbsp; Full async/await support with tokio integration for modern Rust applications. All operations are non-blocking and thread-safe.
    </li>
    <li>
        <b>Schema Validation:</b> &nbsp; Built-in schema validation to catch configuration errors early and ensure type safety.
    </li>
</ul>

<br><br>



## Usage

You can use `noml` both as a command-line tool for validating and converting configuration files, and as a library in your Rust projects.

### **Command-Line Interface (CLI)**

The `noml` CLI provides a simple way to work with NOML files.

<br>

**Validate a NOML file:**
```bash
noml validate config.noml
```

**Parse and display the structure of a NOML file:**

```bash
noml parse app.noml
```

**Show version information:**

```bash
noml version
```

-----

<br>

### **In your Rust project**

To use `noml` in your project, add it as a dependency in your `Cargo.toml`:

```toml
[dependencies]
noml = "0.4.0"
```

You can also enable the `chrono` feature for date/time support and the `async` feature for async operations:

```toml
[dependencies]
noml = { version = "0.4.0", features = ["chrono", "async"] }
```

-----

<br>

## Examples

Here are some examples of how to use the `noml` library in your Rust code.

### **Basic Parsing**

You can easily parse a NOML string and access its values.

```rust
use noml::parse;

let source = r#"
    name = "my-app"
    version = "1.0.0"
    debug = true

    [server]
    host = "localhost"
    port = 8080
"#;

let config = parse(source)?;

// Access top-level values
assert_eq!(config.get("name").unwrap().as_string()?, "my-app");
assert_eq!(config.get("debug").unwrap().as_bool()?, true);

// Access nested values
assert_eq!(config.get("server.host").unwrap().as_string()?, "localhost");
assert_eq!(config.get("server.port").unwrap().as_integer()?, 8080);
```

-----

<br>

### **Using Environment Variables**

`noml` can pull values from environment variables using the `env()` function, with an optional default value.

```rust
use noml::parse;
use std::env;

// Set an environment variable for the test
env::set_var("DATABASE_URL", "postgres://user:pass@host/db");

let source = r#"
    db_url = env("DATABASE_URL")
    secret_key = env("API_KEY", "default-secret")
"#;

let config = parse(source)?;

assert_eq!(config.get("db_url").unwrap().as_string()?, "postgres://user:pass@host/db");
assert_eq!(config.get("secret_key").unwrap().as_string()?, "default-secret");
```

-----

<br>

### **HTTP Includes (Async Feature)**

With the `async` feature enabled, you can include configuration from remote HTTP/HTTPS URLs:

```rust
use noml::parse_from_file_async;

// config.noml content:
// base_config = include "https://config.example.com/app-defaults.noml"
// api_key = env("API_KEY")
// debug = true

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = parse_from_file_async("config.noml").await?;
    
    // The remote config is automatically fetched and merged
    println!("Loaded config with remote includes: {:?}", config);
    Ok(())
}
```

**HTTP Includes Features:**
- ✅ **Secure HTTPS Support**: Full support for HTTPS URLs with proper certificate validation
- ⚡ **Automatic Caching**: Remote configs are cached to improve performance and reduce network requests
- 🔒 **Timeout Protection**: Configurable request timeouts prevent hanging operations
- 🔄 **Error Handling**: Clear error messages for network issues, HTTP errors, and parse failures
- 📦 **No Nested HTTP**: HTTP includes cannot contain other HTTP includes (prevents security issues)

```toml
[dependencies]
noml = { version = "0.4.0", features = ["async"] }
tokio = { version = "1.0", features = ["full"] }
```

-----

<br>

### **Working with Native Types**

`noml` supports special native types for common configuration values like file sizes, durations, IP addresses, and more.

```rust
use noml::parse;

let source = r#"
    # File sizes and durations
    max_upload_size = @size("256MB")
    request_timeout = @duration("90s")
    
    # Network and web
    website = @url("https://example.com")
    server_ip = @ip("192.168.1.100")
    
    # Data formats
    app_version = @semver("2.1.0")
    secret_data = @base64("SGVsbG8gV29ybGQ=")
    user_id = @uuid("550e8400-e29b-41d4-a716-446655440000")
"#;

let config = parse(source)?;

// The values are parsed and validated
assert_eq!(config.get("max_upload_size").unwrap().as_integer()?, 256 * 1024 * 1024);
assert_eq!(config.get("request_timeout").unwrap().as_float()?, 90.0);
assert_eq!(config.get("website").unwrap().as_string()?, "https://example.com");
assert_eq!(config.get("server_ip").unwrap().as_string()?, "192.168.1.100");
assert_eq!(config.get("app_version").unwrap().as_string()?, "2.1.0");
```

**Available Native Types:**
- `@size("10MB")` - File/memory sizes (KB, MB, GB, etc.)
- `@duration("30s")` - Time durations (s, m, h, d)
- `@url("https://...")` - URL validation
- `@ip("192.168.1.1")` - IP address validation (IPv4/IPv6)
- `@semver("1.2.3")` - Semantic version parsing
- `@base64("SGVsbG8=")` - Base64 encoded data
- `@uuid("550e8400-...")` - UUID format validation

-----

<br>

### **Working with Arrays and Tables**

`noml` fully supports arrays, inline tables, and arrays of tables, similar to TOML.

```rust
use noml::parse;

let source = r#"
    # An array of strings
    allowed_roles = ["admin", "editor", "viewer"]

    # An inline table
    point = { x = 1.0, y = -1.0 }

    # An array of tables
    [[users]]
    name = "Alice"
    email = "alice@example.com"

    [[users]]
    name = "Bob"
    email = "bob@example.com"
"#;

let config = parse(source)?;

// Access array elements
let roles = config.get("allowed_roles").unwrap().as_array()?;
assert_eq!(roles.len(), 3);
assert_eq!(roles[0].as_string()?, "admin");

// Access inline table values
assert_eq!(config.get("point.x").unwrap().as_float()?, 1.0);

// Access values from an array of tables
let users = config.get("users").unwrap().as_array()?;
assert_eq!(users[0].get("name").unwrap().as_string()?, "Alice");
assert_eq!(users[1].get("name").unwrap().as_string()?, "Bob");
```

-----

<br>

### **High-Level Configuration Management**

For more advanced use cases, the `Config` struct provides a high-level API for loading, modifying, and saving configurations.

```rust
use noml::Config;
use std::fs;

// Create a temporary file for the example
let temp_dir = tempfile::tempdir()?;
let file_path = temp_dir.path().join("config.noml");
fs::write(&file_path, "version = \"1.0.0\"")?;

// Load the configuration from a file
let mut config = Config::from_file(&file_path)?;
assert_eq!(config.get("version").unwrap().as_string()?, "1.0.0");

// Modify the configuration
config.set("debug", true)?;
config.set("database.port", 5432)?;

// Save the changes back to the file
config.save()?;

// Verify the changes
let updated_config = Config::from_file(&file_path)?;
assert_eq!(updated_config.get("debug").unwrap().as_bool()?, true);
assert_eq!(updated_config.get("database.port").unwrap().as_integer()?, 5432);
```

-----

<br>

### **Async Support** 🚀

`noml` supports async operations for modern Rust applications! Enable the `async` feature:

```toml
[dependencies]
noml = { version = "0.4.0", features = ["async"] }
tokio = { version = "1.0", features = ["full"] }
```

All parsing and file operations are available in async variants:

```rust
use noml::{parse_async, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse configurations asynchronously
    let config = parse_async(r#"
        app_name = "AsyncApp"
        port = env("PORT", "8080")
        
        [database]
        url = env("DATABASE_URL", "sqlite:memory:")
    "#).await?;
    
    println!("App: {}", config.get("app_name")?.as_string()?);

    // Load, modify, and save configurations asynchronously
    let mut config = Config::load_async("config.noml").await?;
    config.set("last_started", "2025-09-19T12:00:00Z")?;
    config.save_async("config.noml").await?;
    
    // Reload to get latest changes
    config.reload_async().await?;
    
    Ok(())
}
```

**Thread Safety**: All NOML types (`Value`, `Config`) are `Send + Sync`, making them safe to share between async tasks and threads. Perfect for concurrent applications and microservices!

**Performance**: Async operations are non-blocking and integrate seamlessly with `tokio`, `async-std`, and other async runtimes.

Run the async demo: `cargo run --example async_demo --features async`

-----

<br>

### **Schema Validation** ✅

`noml` includes built-in schema validation to catch configuration errors early:

```rust
use noml::{Config, Schema, FieldType, SchemaBuilder};

// Load your configuration
let config = Config::from_string(r#"
    app_name = "MyApp"
    port = 8080
    debug = true
    
    [database]
    host = "localhost"
    max_connections = 100
"#)?;

// Define expected schema
let schema = SchemaBuilder::new()
    .require_string("app_name")
    .require_integer("port")
    .optional_bool("debug")
    .build();

// Validate configuration against schema
config.validate_schema(&schema)?;

// Or create more complex schemas
let db_schema = Schema::new()
    .required_field("host", FieldType::String)
    .required_field("max_connections", FieldType::Integer)
    .allow_additional(false);

let app_schema = Schema::new()
    .required_field("app_name", FieldType::String)
    .required_field("port", FieldType::Integer)
    .required_field("database", FieldType::Table(db_schema));

config.validate_schema(&app_schema)?;
```

**Benefits:**
- 🛡️ **Early Error Detection**: Catch configuration issues before runtime
- 🎯 **Type Safety**: Ensure values are the expected types
- 📋 **Required Fields**: Validate that critical configuration is present
- 🔍 **Clear Error Messages**: Detailed validation failure reports



<br>