<div id="top" align="center">
    <h1>
        <span>NOML +&nbsp; RUST</span>
        <br>
        <sub><sup>API REFERENCE</sup></sub>
    </h1>
</div>

## Table of Contents

- **[Installation](#installation)**
- **[Feature Flags](#feature-flags)**
- **[Core Parsing Functions](#core-parsing-functions)**
  - **[parse()](#parse)**
  - **[parse_from_file()](#parse_from_file)**
  - **[validate()](#validate)**
  - **[parse_raw()](#parse_raw)**
  - **[parse_raw_from_file()](#parse_raw_from_file)**
- **[Format Preservation API](#format-preservation-api)**
  - **[parse_preserving()](#parse_preserving)**
  - **[parse_preserving_from_file()](#parse_preserving_from_file)**
  - **[modify_preserving()](#modify_preserving)**
  - **[save_preserving()](#save_preserving)**
- **[Async Functions](#async-functions)**
  - **[parse_async()](#parse_async)**
  - **[parse_from_file_async()](#parse_from_file_async)**
  - **[parse_raw_from_file_async()](#parse_raw_from_file_async)**
- **[Configuration Management](#configuration-management)**
  - **[Config::new()](#config_new)**
  - **[Config::from_string()](#config_from_string)**
  - **[Config::from_file()](#config_from_file)**
  - **[Config::builder()](#config_builder)**
  - **[Config::get()](#config_get)**
  - **[Config::get_or()](#config_get_or)**
  - **[Config::get_or_insert()](#config_get_or_insert)**
  - **[Config::set()](#config_set)**
  - **[Config::remove()](#config_remove)**
  - **[Config::contains_key()](#config_contains_key)**
  - **[Config::keys()](#config_keys)**
  - **[Config::save()](#config_save)**
  - **[Config::save_to_file()](#config_save_to_file)**
  - **[Config::merge()](#config_merge)**
  - **[Config::load_async()](#config_load_async)**
  - **[Config::save_async()](#config_save_async)**
  - **[Config::reload_async()](#config_reload_async)**
- **[Configuration Builder](#configuration-builder)**
  - **[ConfigBuilder::default_value()](#configbuilder_default_value)**
  - **[ConfigBuilder::allow_missing()](#configbuilder_allow_missing)**
  - **[ConfigBuilder::validate()](#configbuilder_validate)**
  - **[ConfigBuilder::build_from_file()](#configbuilder_build_from_file)**
  - **[ConfigBuilder::build_from_string()](#configbuilder_build_from_string)**
- **[Value System](#value-system)**
  - **[Value::get()](#value_get)**
  - **[Value::as_string()](#value_as_string)**
  - **[Value::as_integer()](#value_as_integer)**
  - **[Value::as_float()](#value_as_float)**
  - **[Value::as_bool()](#value_as_bool)**
  - **[Value::as_array()](#value_as_array)**
  - **[Value::as_table()](#value_as_table)**
- **[Schema Validation](#schema-validation)**
  - **[Schema::new()](#schema_new)**
  - **[SchemaBuilder::new()](#schemabuilder_new)**
  - **[Config::validate_schema()](#config_validate_schema)**
- **[Macros](#macros)**
  - **[noml_value!](#noml_value_macro)**

<hr>
<br>

<h2 id="installation">Installation</h2>

### Install Manually
Add this to your `Cargo.toml`:
```toml
[dependencies]
noml = "0.9.0"

# Optional features
[dependencies.noml]
version = "0.9.0"
features = ["async", "chrono"]
```

### Install via Terminal
```bash
# Basic installation
cargo add noml

# With async support
cargo add noml --features async

# With all features
cargo add noml --features async,chrono
```

<br>
<hr>
<a href="#top">&uarr; <b>TOP</b></a>
<br>
<br>

<h2 id="feature-flags">Feature Flags</h2>

| Feature               | Default | Description |
|----------------------|:-------:|-------------|
| `async`              |  ❌     | Enables async functions for file operations and HTTP includes |
| `chrono`             |  ❌     | Enables DateTime support with chrono integration |

<br>
<hr>
<a href="#top">&uarr; <b>TOP</b></a>
<br>
<br>

<h2 id="core-parsing-functions">Core Parsing Functions</h2>

<h3 id="parse">parse()</h3>

**Function Signature:**
```rust
pub fn parse(source: &str) -> Result<Value>
```

**Parameters:**
- `source: &str` - NOML configuration text to parse

**Returns:**
- `Result<Value>` - Resolved configuration data or `NomlError`

**Description:**
Main entry point for parsing NOML configuration. Performs complete parsing and resolution including environment variables, file includes, interpolation, and native types.

**Examples:**

Basic parsing:
```rust
use noml::parse;

let config = parse(r#"
    app_name = "my-service"
    port = 8080
    debug = true
    
    [database]
    host = "localhost"
    max_connections = 100
"#)?;

assert_eq!(config.get("app_name")?.as_string()?, "my-service");
assert_eq!(config.get("database.host")?.as_string()?, "localhost");
```

With environment variables and native types:
```rust
use noml::parse;
use std::env;

env::set_var("APP_PORT", "3000");

let config = parse(r#"
    name = "web-server"
    port = env("APP_PORT", 8080)
    timeout = @duration("30s")
    max_size = @size("10MB")
"#)?;

assert_eq!(config.get("port")?.as_integer()?, 3000);
```

<h3 id="parse_from_file">parse_from_file()</h3>

**Function Signature:**
```rust
pub fn parse_from_file<P: AsRef<Path>>(path: P) -> Result<Value>
```

**Parameters:**
- `path: P` - File path (anything implementing `AsRef<Path>`)

**Returns:**
- `Result<Value>` - Resolved configuration data or `NomlError`

**Description:**
Parse NOML configuration from a file with full resolution. Sets the base path for relative includes.

**Examples:**

```rust
use noml::parse_from_file;

// Parse from file
let config = parse_from_file("config.noml")?;
assert_eq!(config.get("app.name")?.as_string()?, "MyApp");

// Works with Path and PathBuf too
use std::path::Path;
let config = parse_from_file(Path::new("configs/app.noml"))?;
```

<h3 id="validate">validate()</h3>

**Function Signature:**
```rust
pub fn validate(source: &str) -> Result<()>
```

**Parameters:**
- `source: &str` - NOML configuration text to validate

**Returns:**
- `Result<()>` - Ok if valid, `NomlError` with details if invalid

**Description:**
Validate NOML configuration syntax without resolving dynamic features. Quick syntax checking.

**Examples:**

```rust
use noml::validate;

// Valid configuration
validate(r#"
    name = "test"
    port = 8080
    [database]
    host = "localhost"
"#)?; // Returns Ok(())

// Invalid configuration
match validate("invalid = [1, 2,") {
    Err(e) => println!("Validation error: {}", e),
    Ok(_) => unreachable!(),
}
```

<h3 id="parse_raw">parse_raw()</h3>

**Function Signature:**
```rust
pub fn parse_raw(source: &str) -> Result<Document>
```

**Parameters:**
- `source: &str` - NOML configuration text to parse

**Returns:**
- `Result<Document>` - Raw AST document without resolution

**Description:**
Parse NOML into raw AST without resolving dynamic features. Useful for tooling and analysis.

**Examples:**

```rust
use noml::parse_raw;

let document = parse_raw(r#"
    name = "test"
    port = env("PORT", 8080)
    include "other.noml"
"#)?;

// Document contains unresolved AST nodes
println!("Raw document: {:?}", document);
```

<h3 id="parse_raw_from_file">parse_raw_from_file()</h3>

**Function Signature:**
```rust
pub fn parse_raw_from_file<P: AsRef<Path>>(path: P) -> Result<Document>
```

**Parameters:**
- `path: P` - File path to parse

**Returns:**
- `Result<Document>` - Raw AST document without resolution

**Description:**
Parse NOML file into raw AST without resolving dynamic features.

**Examples:**

```rust
use noml::parse_raw_from_file;

let document = parse_raw_from_file("config.noml")?;
// Raw AST with source location information preserved
```

<br>
<hr>
<a href="#top">&uarr; <b>TOP</b></a>
<br>
<br>

<h2 id="format-preservation-api">Format Preservation API</h2>

NOML's revolutionary format preservation system - the first configuration language with true format preservation!

<h3 id="parse_preserving">parse_preserving()</h3>

**Function Signature:**
```rust
pub fn parse_preserving(input: &str) -> Result<Document>
```

**Parameters:**
- `input: &str` - NOML configuration text to parse

**Returns:**
- `Result<Document>` - Document with complete format metadata preserved

**Description:**
Parse NOML with full format preservation including whitespace, comments, indentation style, and quote styles. Enables perfect round-trip editing.

**Examples:**

```rust
use noml::parse_preserving;

let source = r#"
# My app config
app_name = "MyApp"    # Main app
version = "1.0.0"

[server]
    host = "localhost"
    port = 8080
"#;

let document = parse_preserving(source)?;
// All comments, whitespace, and formatting preserved!
```

<h3 id="parse_preserving_from_file">parse_preserving_from_file()</h3>

**Function Signature:**
```rust
pub fn parse_preserving_from_file<P: AsRef<Path>>(path: P) -> Result<Document>
```

**Parameters:**
- `path: P` - File path to parse with format preservation

**Returns:**
- `Result<Document>` - Document with complete format metadata

**Description:**
Parse NOML file with complete format preservation for zero-loss editing.

**Examples:**

```rust
use noml::parse_preserving_from_file;

let document = parse_preserving_from_file("config.noml")?;
// Perfect format preservation from file
```

<h3 id="modify_preserving">modify_preserving()</h3>

**Function Signature:**
```rust
pub fn modify_preserving(document: &Document, modifications: impl Fn(&mut Value) -> Result<()>) -> Result<String>
```

**Parameters:**
- `document: &Document` - Document with format metadata
- `modifications: impl Fn(&mut Value) -> Result<()>` - Function to modify values

**Returns:**
- `Result<String>` - Modified configuration with preserved formatting

**Description:**
Modify configuration values while preserving original formatting, comments, and style.

**Examples:**

```rust
use noml::{parse_preserving, modify_preserving};

let document = parse_preserving(r#"
# Server configuration  
[server]
host = "localhost"    # Default host
port = 8080          # Default port
"#)?;

let modified = modify_preserving(&document, |config| {
    config.set("server.port", 9000)?;
    config.set("server.ssl", true)?;
    Ok(())
})?;

// Result preserves comments and formatting!
// Only the values changed, everything else intact
```

<h3 id="save_preserving">save_preserving()</h3>

**Function Signature:**
```rust
pub fn save_preserving<P: AsRef<Path>>(document: &Document, values: &Value, path: P) -> Result<()>
```

**Parameters:**
- `document: &Document` - Original document with format metadata
- `values: &Value` - Modified configuration values
- `path: P` - File path to save to

**Returns:**
- `Result<()>` - Ok if saved successfully

**Description:**
Save modified configuration with perfect format preservation to file.

**Examples:**

```rust
use noml::{parse_preserving_from_file, save_preserving};

let document = parse_preserving_from_file("config.noml")?;
let mut values = document.resolve()?;

// Make changes
values.set("app.version", "2.0.0")?;

// Save with perfect format preservation
save_preserving(&document, &values, "config.noml")?;
```

<br>
<hr>
<a href="#top">&uarr; <b>TOP</b></a>
<br>
<br>

<h2 id="async-functions">Async Functions</h2>

Available with the `async` feature flag.

<h3 id="parse_async">parse_async()</h3>

**Function Signature:**
```rust
#[cfg(feature = "async")]
pub async fn parse_async(source: &str) -> Result<Value>
```

**Parameters:**
- `source: &str` - NOML configuration text to parse

**Returns:**
- `Result<Value>` - Resolved configuration data

**Description:**
Async version of `parse()` that can handle HTTP includes and other async operations.

**Examples:**

```rust
use noml::parse_async;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = parse_async(r#"
        name = "async-app"
        include "https://api.example.com/config.noml"
    "#).await?;
    
    Ok(())
}
```

<h3 id="parse_from_file_async">parse_from_file_async()</h3>

**Function Signature:**
```rust
#[cfg(feature = "async")]
pub async fn parse_from_file_async<P: AsRef<Path>>(path: P) -> Result<Value>
```

**Parameters:**
- `path: P` - File path to parse

**Returns:**
- `Result<Value>` - Resolved configuration data

**Description:**
Async version of `parse_from_file()` for non-blocking file I/O.

**Examples:**

```rust
use noml::parse_from_file_async;

let config = parse_from_file_async("config.noml").await?;
```

<h3 id="parse_raw_from_file_async">parse_raw_from_file_async()</h3>

**Function Signature:**
```rust
#[cfg(feature = "async")]
pub async fn parse_raw_from_file_async<P: AsRef<Path>>(path: P) -> Result<Document>
```

**Parameters:**
- `path: P` - File path to parse

**Returns:**
- `Result<Document>` - Raw AST document

**Description:**
Async version of `parse_raw_from_file()` for non-blocking raw parsing.

**Examples:**

```rust
use noml::parse_raw_from_file_async;

let document = parse_raw_from_file_async("config.noml").await?;
```

<br>
<hr>
<a href="#top">&uarr; <b>TOP</b></a>
<br>
<br>

<h2 id="configuration-management">Configuration Management</h2>

The `Config` struct provides high-level configuration management with modification tracking.

<h3 id="config_new">Config::new()</h3>

**Function Signature:**
```rust
pub fn new() -> Self
```

**Returns:**
- `Config` - Empty configuration

**Description:**
Create a new empty configuration.

**Examples:**

```rust
use noml::Config;

let mut config = Config::new();
config.set("app.name", "NewApp")?;
```

<h3 id="config_from_string">Config::from_string()</h3>

**Function Signature:**
```rust
pub fn from_string(content: &str) -> Result<Self>
```

**Parameters:**
- `content: &str` - NOML configuration text

**Returns:**
- `Result<Config>` - Configuration instance

**Description:**
Load configuration from a string with modification tracking.

**Examples:**

```rust
use noml::Config;

let config = Config::from_string(r#"
    name = "MyApp"
    version = "1.0.0"
    
    [database]
    host = "localhost"
    port = 5432
"#)?;

assert_eq!(config.get("name").unwrap().as_string().unwrap(), "MyApp");
```

<h3 id="config_from_file">Config::from_file()</h3>

**Function Signature:**
```rust
pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self>
```

**Parameters:**
- `path: P` - File path to load

**Returns:**
- `Result<Config>` - Configuration instance

**Description:**
Load configuration from a file with source tracking.

**Examples:**

```rust
use noml::Config;

let config = Config::from_file("config.noml")?;
println!("Loaded from: {:?}", config.source_path());
```

<h3 id="config_builder">Config::builder()</h3>

**Function Signature:**
```rust
pub fn builder() -> ConfigBuilder
```

**Returns:**
- `ConfigBuilder` - Builder for advanced configuration loading

**Description:**
Create a configuration builder for advanced loading with defaults and validation.

**Examples:**

```rust
use noml::Config;

let config = Config::builder()
    .default_value("debug", false)
    .default_value("port", 8080)
    .allow_missing(true)
    .build_from_file("config.noml")?;
```

<h3 id="config_get">Config::get()</h3>

**Function Signature:**
```rust
pub fn get(&self, key: &str) -> Option<&Value>
```

**Parameters:**
- `key: &str` - Dot-separated key path

**Returns:**
- `Option<&Value>` - Value reference or None if not found

**Description:**
Get a value by key path. Supports nested access with dot notation.

**Examples:**

```rust
use noml::Config;

let config = Config::from_string(r#"
    [database]
    host = "localhost"
    port = 5432
"#)?;

let host = config.get("database.host").unwrap();
assert_eq!(host.as_string().unwrap(), "localhost");

let port = config.get("database.port").unwrap();
assert_eq!(port.as_integer().unwrap(), 5432);
```

<h3 id="config_get_or">Config::get_or()</h3>

**Function Signature:**
```rust
pub fn get_or<T>(&self, key: &str, default: T) -> Result<&Value>
where T: Into<Value>
```

**Parameters:**
- `key: &str` - Key path to get
- `default: T` - Default value if key not found

**Returns:**
- `Result<&Value>` - Value reference or default

**Description:**
Get a value with a fallback default (conceptual - returns the existing value or error).

**Examples:**

```rust
use noml::Config;

let config = Config::from_string(r#"
    [server]
    port = 8080
"#)?;

// Key exists
let port = config.get_or("server.port", 3000)?;
assert_eq!(port.as_integer().unwrap(), 8080);
```

<h3 id="config_get_or_insert">Config::get_or_insert()</h3>

**Function Signature:**
```rust
pub fn get_or_insert<T>(&mut self, key: &str, default: T) -> Result<&Value>
where T: Into<Value>
```

**Parameters:**
- `key: &str` - Key path
- `default: T` - Default value to insert if missing

**Returns:**
- `Result<&Value>` - Value reference (existing or newly inserted)

**Description:**
Get a value or insert a default if the key doesn't exist.

**Examples:**

```rust
use noml::Config;

let mut config = Config::from_string("name = 'app'")?;

// Insert default if missing
let debug = config.get_or_insert("debug", false)?;
assert_eq!(debug.as_bool().unwrap(), false);

// Now the key exists
assert!(config.contains_key("debug"));
```

<h3 id="config_set">Config::set()</h3>

**Function Signature:**
```rust
pub fn set<T>(&mut self, key: &str, value: T) -> Result<()>
where T: Into<Value>
```

**Parameters:**
- `key: &str` - Dot-separated key path
- `value: T` - Value to set (any type convertible to Value)

**Returns:**
- `Result<()>` - Ok if successful

**Description:**
Set a value at the given key path. Creates nested structures as needed.

**Examples:**

```rust
use noml::Config;

let mut config = Config::new();

// Set simple values
config.set("app.name", "MyApp")?;
config.set("app.version", "1.0.0")?;
config.set("server.port", 8080)?;
config.set("server.debug", true)?;

assert_eq!(config.get("server.port").unwrap().as_integer().unwrap(), 8080);
```

<h3 id="config_remove">Config::remove()</h3>

**Function Signature:**
```rust
pub fn remove(&mut self, key: &str) -> Result<Option<Value>>
```

**Parameters:**
- `key: &str` - Key path to remove

**Returns:**
- `Result<Option<Value>>` - Removed value or None if not found

**Description:**
Remove a value by key path and return the removed value.

**Examples:**

```rust
use noml::Config;

let mut config = Config::from_string(r#"
    name = "test"
    version = "1.0"
    debug = true
"#)?;

// Remove a key
let removed = config.remove("debug")?;
assert_eq!(removed.unwrap().as_bool().unwrap(), true);

// Key no longer exists
assert!(!config.contains_key("debug"));
```

<h3 id="config_contains_key">Config::contains_key()</h3>

**Function Signature:**
```rust
pub fn contains_key(&self, key: &str) -> bool
```

**Parameters:**
- `key: &str` - Key path to check

**Returns:**
- `bool` - True if key exists

**Description:**
Check if a key exists in the configuration.

**Examples:**

```rust
use noml::Config;

let config = Config::from_string(r#"
    [database]
    host = "localhost"
"#)?;

assert!(config.contains_key("database.host"));
assert!(!config.contains_key("database.port"));
```

<h3 id="config_keys">Config::keys()</h3>

**Function Signature:**
```rust
pub fn keys(&self) -> Vec<String>
```

**Returns:**
- `Vec<String>` - List of all top-level keys

**Description:**
Get all top-level keys in the configuration.

**Examples:**

```rust
use noml::Config;

let config = Config::from_string(r#"
    name = "app"
    version = "1.0"
    [database]
    host = "localhost"
"#)?;

let keys = config.keys();
// Contains: ["name", "version", "database"]
assert!(keys.contains(&"name".to_string()));
assert!(keys.contains(&"database".to_string()));
```

<h3 id="config_save">Config::save()</h3>

**Function Signature:**
```rust
pub fn save(&self) -> Result<()>
```

**Returns:**
- `Result<()>` - Ok if saved successfully

**Description:**
Save configuration back to its source file (if loaded from file).

**Examples:**

```rust
use noml::Config;

let mut config = Config::from_file("config.noml")?;
config.set("app.version", "2.0.0")?;

// Save back to original file
config.save()?;
```

<h3 id="config_save_to_file">Config::save_to_file()</h3>

**Function Signature:**
```rust
pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()>
```

**Parameters:**
- `path: P` - File path to save to

**Returns:**
- `Result<()>` - Ok if saved successfully

**Description:**
Save configuration to a specified file.

**Examples:**

```rust
use noml::Config;

let config = Config::from_string(r#"
    name = "MyApp"
    version = "1.0.0"
"#)?;

config.save_to_file("output.noml")?;
```

<h3 id="config_merge">Config::merge()</h3>

**Function Signature:**
```rust
pub fn merge(&mut self, other: &Config) -> Result<()>
```

**Parameters:**
- `other: &Config` - Configuration to merge in

**Returns:**
- `Result<()>` - Ok if merged successfully

**Description:**
Merge another configuration into this one. Values from `other` take precedence.

**Examples:**

```rust
use noml::Config;

let mut base = Config::from_string(r#"
    name = "app"
    version = "1.0"
    debug = false
"#)?;

let override_config = Config::from_string(r#"
    version = "2.0"
    debug = true
    new_field = "added"
"#)?;

base.merge(&override_config)?;

assert_eq!(base.get("version").unwrap().as_string().unwrap(), "2.0");
assert!(base.get("debug").unwrap().as_bool().unwrap());
assert_eq!(base.get("new_field").unwrap().as_string().unwrap(), "added");
```

<h3 id="config_load_async">Config::load_async()</h3>

**Function Signature:**
```rust
#[cfg(feature = "async")]
pub async fn load_async<P: AsRef<Path>>(path: P) -> Result<Self>
```

**Parameters:**
- `path: P` - File path to load

**Returns:**
- `Result<Config>` - Configuration instance

**Description:**
Async version of `from_file()` for non-blocking file loading.

**Examples:**

```rust
use noml::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load_async("config.noml").await?;
    Ok(())
}
```

<h3 id="config_save_async">Config::save_async()</h3>

**Function Signature:**
```rust
#[cfg(feature = "async")]
pub async fn save_async(&self) -> Result<()>
```

**Returns:**
- `Result<()>` - Ok if saved successfully

**Description:**
Async version of `save()` for non-blocking file writing.

**Examples:**

```rust
use noml::Config;

let mut config = Config::load_async("config.noml").await?;
config.set("updated", true)?;
config.save_async().await?;
```

<h3 id="config_reload_async">Config::reload_async()</h3>

**Function Signature:**
```rust
#[cfg(feature = "async")]
pub async fn reload_async(&mut self) -> Result<()>
```

**Returns:**
- `Result<()>` - Ok if reloaded successfully

**Description:**
Reload configuration from its source file asynchronously.

**Examples:**

```rust
use noml::Config;

let mut config = Config::load_async("config.noml").await?;
// ... time passes, file changes ...
config.reload_async().await?; // Fresh data from file
```

<br>
<hr>
<a href="#top">&uarr; <b>TOP</b></a>
<br>
<br>

<h2 id="configuration-builder">Configuration Builder</h2>

The `ConfigBuilder` provides advanced configuration loading with defaults and validation.

<h3 id="configbuilder_default_value">ConfigBuilder::default_value()</h3>

**Function Signature:**
```rust
pub fn default_value<T>(mut self, key: &str, value: T) -> Self
where T: Into<Value>
```

**Parameters:**
- `key: &str` - Key path for default value
- `value: T` - Default value

**Returns:**
- `Self` - Builder for chaining

**Description:**
Add a default value for a key. Applied if the key is missing after loading.

**Examples:**

```rust
use noml::Config;

let config = Config::builder()
    .default_value("debug", false)
    .default_value("port", 8080)
    .default_value("host", "localhost")
    .build_from_string(r#"
        name = "MyApp"
        port = 9000
    "#)?;

// 'port' from config (9000), others from defaults
assert_eq!(config.get("port").unwrap().as_integer().unwrap(), 9000);
assert_eq!(config.get("host").unwrap().as_string().unwrap(), "localhost");
assert!(!config.get("debug").unwrap().as_bool().unwrap());
```

<h3 id="configbuilder_allow_missing">ConfigBuilder::allow_missing()</h3>

**Function Signature:**
```rust
pub fn allow_missing(mut self, allow: bool) -> Self
```

**Parameters:**
- `allow: bool` - Whether to allow missing files

**Returns:**
- `Self` - Builder for chaining

**Description:**
Control whether missing config files should cause errors or return empty configs.

**Examples:**

```rust
use noml::Config;

// This won't error even if file doesn't exist
let config = Config::builder()
    .allow_missing(true)
    .default_value("name", "DefaultApp")
    .build_from_file("maybe_missing.noml")?;

// Will have default values even if file was missing
assert_eq!(config.get("name").unwrap().as_string().unwrap(), "DefaultApp");
```

<h3 id="configbuilder_validate">ConfigBuilder::validate()</h3>

**Function Signature:**
```rust
pub fn validate(mut self, validate: bool) -> Self
```

**Parameters:**
- `validate: bool` - Whether to enable validation

**Returns:**
- `Self` - Builder for chaining

**Description:**
Enable or disable validation during configuration building.

**Examples:**

```rust
use noml::Config;

let config = Config::builder()
    .validate(true)
    .build_from_string("name = 'ValidApp'")?;
```

<h3 id="configbuilder_build_from_file">ConfigBuilder::build_from_file()</h3>

**Function Signature:**
```rust
pub fn build_from_file<P: AsRef<Path>>(self, path: P) -> Result<Config>
```

**Parameters:**
- `path: P` - File path to load

**Returns:**
- `Result<Config>` - Built configuration

**Description:**
Build configuration from a file with all builder options applied.

**Examples:**

```rust
use noml::Config;

let config = Config::builder()
    .default_value("debug", false)
    .allow_missing(true)
    .build_from_file("config.noml")?;
```

<h3 id="configbuilder_build_from_string">ConfigBuilder::build_from_string()</h3>

**Function Signature:**
```rust
pub fn build_from_string(self, content: &str) -> Result<Config>
```

**Parameters:**
- `content: &str` - NOML configuration text

**Returns:**
- `Result<Config>` - Built configuration

**Description:**
Build configuration from a string with all builder options applied.

**Examples:**

```rust
use noml::Config;

let config = Config::builder()
    .default_value("timeout", 30)
    .build_from_string(r#"
        name = "MyApp"
        # timeout will be 30 (default)
    "#)?;

assert_eq!(config.get("timeout").unwrap().as_integer().unwrap(), 30);
```

<br>
<hr>
<a href="#top">&uarr; <b>TOP</b></a>
<br>
<br>

<h2 id="value-system">Value System</h2>

The `Value` enum represents all NOML data types with convenient accessor methods.

<h3 id="value_get">Value::get()</h3>

**Function Signature:**
```rust
pub fn get(&self, key: &str) -> Option<&Value>
```

**Parameters:**
- `key: &str` - Dot-separated key path

**Returns:**
- `Option<&Value>` - Value reference or None

**Description:**
Access nested values using dot notation (e.g., "server.database.host").

**Examples:**

```rust
use noml::parse;

let config = parse(r#"
    [server]
    port = 8080
    
    [server.database]
    host = "localhost"
    credentials = { user = "admin", pass = "secret" }
"#)?;

// Nested access
assert_eq!(config.get("server.port")?.as_integer()?, 8080);
assert_eq!(config.get("server.database.host")?.as_string()?, "localhost");
assert_eq!(config.get("server.database.credentials.user")?.as_string()?, "admin");
```

<h3 id="value_as_string">Value::as_string()</h3>

**Function Signature:**
```rust
pub fn as_string(&self) -> Result<String>
```

**Returns:**
- `Result<String>` - String value or conversion error

**Description:**
Convert value to string. Supports automatic conversion from other types.

**Examples:**

```rust
use noml::parse;

let config = parse(r#"
    name = "MyApp"
    version = "1.0.0"
    port_str = "8080"
    debug_str = "true"
"#)?;

assert_eq!(config.get("name")?.as_string()?, "MyApp");
assert_eq!(config.get("version")?.as_string()?, "1.0.0");

// String representations
assert_eq!(config.get("port_str")?.as_string()?, "8080");
assert_eq!(config.get("debug_str")?.as_string()?, "true");
```

<h3 id="value_as_integer">Value::as_integer()</h3>

**Function Signature:**
```rust
pub fn as_integer(&self) -> Result<i64>
```

**Returns:**
- `Result<i64>` - Integer value or conversion error

**Description:**
Convert value to integer. Supports conversion from strings and exact floats.

**Examples:**

```rust
use noml::parse;

let config = parse(r#"
    port = 8080
    max_connections = 100
    string_number = "42"
    exact_float = 10.0
"#)?;

assert_eq!(config.get("port")?.as_integer()?, 8080);
assert_eq!(config.get("max_connections")?.as_integer()?, 100);

// Automatic conversions
assert_eq!(config.get("string_number")?.as_integer()?, 42);
assert_eq!(config.get("exact_float")?.as_integer()?, 10);
```

<h3 id="value_as_float">Value::as_float()</h3>

**Function Signature:**
```rust
pub fn as_float(&self) -> Result<f64>
```

**Returns:**
- `Result<f64>` - Float value or conversion error

**Description:**
Convert value to float. Supports conversion from integers and strings.

**Examples:**

```rust
use noml::parse;

let config = parse(r#"
    pi = 3.14159
    temperature = 98.6
    integer_as_float = 42
    string_float = "2.5"
"#)?;

assert_eq!(config.get("pi")?.as_float()?, 3.14159);
assert_eq!(config.get("temperature")?.as_float()?, 98.6);

// Automatic conversions
assert_eq!(config.get("integer_as_float")?.as_float()?, 42.0);
assert_eq!(config.get("string_float")?.as_float()?, 2.5);
```

<h3 id="value_as_bool">Value::as_bool()</h3>

**Function Signature:**
```rust
pub fn as_bool(&self) -> Result<bool>
```

**Returns:**
- `Result<bool>` - Boolean value or conversion error

**Description:**
Convert value to boolean. Supports conversion from strings ("true"/"false").

**Examples:**

```rust
use noml::parse;

let config = parse(r#"
    debug = true
    production = false
    string_true = "true"
    string_false = "false"
"#)?;

assert!(config.get("debug")?.as_bool()?);
assert!(!config.get("production")?.as_bool()?);

// String conversions
assert!(config.get("string_true")?.as_bool()?);
assert!(!config.get("string_false")?.as_bool()?);
```

<h3 id="value_as_array">Value::as_array()</h3>

**Function Signature:**
```rust
pub fn as_array(&self) -> Result<&Vec<Value>>
```

**Returns:**
- `Result<&Vec<Value>>` - Array reference or type error

**Description:**
Get value as an array reference. No automatic conversion.

**Examples:**

```rust
use noml::parse;

let config = parse(r#"
    features = ["auth", "logging", "metrics"]
    ports = [8080, 8081, 8082]
    mixed = [1, "two", true, null]
"#)?;

let features = config.get("features")?.as_array()?;
assert_eq!(features.len(), 3);
assert_eq!(features[0].as_string()?, "auth");

let ports = config.get("ports")?.as_array()?;
assert_eq!(ports[1].as_integer()?, 8081);

let mixed = config.get("mixed")?.as_array()?;
assert_eq!(mixed[0].as_integer()?, 1);
assert_eq!(mixed[1].as_string()?, "two");
assert!(mixed[2].as_bool()?);
```

<h3 id="value_as_table">Value::as_table()</h3>

**Function Signature:**
```rust
pub fn as_table(&self) -> Result<&BTreeMap<String, Value>>
```

**Returns:**
- `Result<&BTreeMap<String, Value>>` - Table reference or type error

**Description:**
Get value as a table (map) reference. No automatic conversion.

**Examples:**

```rust
use noml::parse;

let config = parse(r#"
    [database]
    host = "localhost"
    port = 5432
    
    inline_table = { x = 10, y = 20 }
"#)?;

let db_table = config.get("database")?.as_table()?;
assert_eq!(db_table.get("host").unwrap().as_string().unwrap(), "localhost");
assert_eq!(db_table.get("port").unwrap().as_integer().unwrap(), 5432);

let inline = config.get("inline_table")?.as_table()?;
assert_eq!(inline.get("x").unwrap().as_integer().unwrap(), 10);
```

<br>
<hr>
<a href="#top">&uarr; <b>TOP</b></a>
<br>
<br>

<h2 id="schema-validation">Schema Validation</h2>

NOML provides comprehensive schema validation for type-safe configuration management.

<h3 id="schema_new">Schema::new()</h3>

**Function Signature:**
```rust
pub fn new() -> Self
```

**Returns:**
- `Schema` - New empty schema

**Description:**
Create a new schema for validation rules.

**Examples:**

```rust
use noml::{Schema, FieldType};

let schema = Schema::new()
    .required_field("name", FieldType::String)
    .required_field("port", FieldType::Integer)
    .optional_field("debug", FieldType::Bool);
```

<h3 id="schemabuilder_new">SchemaBuilder::new()</h3>

**Function Signature:**
```rust
pub fn new() -> Self
```

**Returns:**
- `SchemaBuilder` - New schema builder

**Description:**
Create a schema builder for convenient schema construction.

**Examples:**

```rust
use noml::SchemaBuilder;

let schema = SchemaBuilder::new()
    .require_string("app_name")
    .require_integer("port")
    .optional_bool("debug")
    .build();
```

<h3 id="config_validate_schema">Config::validate_schema()</h3>

**Function Signature:**
```rust
pub fn validate_schema(&self, schema: &Schema) -> Result<()>
```

**Parameters:**
- `schema: &Schema` - Schema to validate against

**Returns:**
- `Result<()>` - Ok if valid, detailed error if invalid

**Description:**
Validate configuration against a schema with detailed error reporting.

**Examples:**

```rust
use noml::{Config, SchemaBuilder};

let config = Config::from_string(r#"
    app_name = "MyApp"
    port = 8080
    debug = true
    
    [database]
    host = "localhost"
    max_connections = 100
"#)?;

let schema = SchemaBuilder::new()
    .require_string("app_name")
    .require_integer("port")
    .optional_bool("debug")
    .build();

// Validate - will succeed
config.validate_schema(&schema)?;

// More complex nested validation
let db_schema = Schema::new()
    .required_field("host", FieldType::String)
    .required_field("max_connections", FieldType::Integer);

let app_schema = Schema::new()
    .required_field("app_name", FieldType::String)
    .required_field("port", FieldType::Integer)
    .required_field("database", FieldType::Table(db_schema));

config.validate_schema(&app_schema)?;
```

<br>
<hr>
<a href="#top">&uarr; <b>TOP</b></a>
<br>
<br>

<h2 id="macros">Macros</h2>

<h3 id="noml_value_macro">noml_value!</h3>

**Macro Signature:**
```rust
macro_rules! noml_value { ... }
```

**Description:**
Create NOML `Value` instances using convenient syntax. Supports all NOML types including nested structures.

**Examples:**

Basic values:
```rust
use noml::{noml_value, Value};

// Primitives
let null_val = noml_value!(null);
let bool_val = noml_value!(true);
let string_val = noml_value!("hello");
let int_val = noml_value!(42);
let float_val = noml_value!(3.15);

assert_eq!(null_val, Value::Null);
assert_eq!(bool_val, Value::Bool(true));
assert_eq!(string_val, Value::String("hello".to_string()));
```

Arrays:
```rust
use noml::noml_value;

let array = noml_value!([1, 2, 3]);
let mixed = noml_value!([1, "hello", true, null]);
let nested = noml_value!([
    [1, 2],
    ["a", "b"],
    [true, false]
]);

assert_eq!(array.as_array().unwrap().len(), 3);
```

Tables (objects):
```rust
use noml::noml_value;

let simple_table = noml_value!({
    "name" => "MyApp",
    "version" => "1.0.0",
    "port" => 8080
});

let nested_config = noml_value!({
    "app" => {
        "name" => "ComplexApp",
        "version" => "2.0.0"
    },
    "server" => {
        "host" => "localhost",
        "port" => 8080,
        "ssl" => true
    },
    "features" => ["auth", "logging", "metrics"],
    "database" => {
        "connections" => [
            {
                "name" => "primary",
                "host" => "db1.example.com",
                "port" => 5432
            },
            {
                "name" => "replica",
                "host" => "db2.example.com", 
                "port" => 5433
            }
        ]
    }
});

// Access nested values
assert_eq!(
    nested_config.get("server.port").unwrap().as_integer().unwrap(),
    8080
);
```

Programmatic configuration building:
```rust
use noml::noml_value;

// Build configuration programmatically
let env = "production";
let port = if env == "production" { 443 } else { 8080 };

let config = noml_value!({
    "environment" => env,
    "server" => {
        "port" => port,
        "ssl" => (env == "production"),
        "host" => "0.0.0.0"
    },
    "features" => match env {
        "production" => vec!["auth", "ssl", "monitoring"],
        "development" => vec!["debug", "hot-reload"],
        _ => vec!["basic"]
    }
});
```

<br>
<hr>
<a href="#top">&uarr; <b>TOP</b></a>
<br>
<br>

## Tutorials

### Basic Configuration Management

```rust
use noml::{Config, parse};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Method 1: Direct parsing
    let config = parse(r#"
        app_name = "MyService"
        port = env("PORT", 8080)
        timeout = @duration("30s")
        
        [database]
        url = env("DATABASE_URL", "sqlite:memory:")
        max_connections = 10
    "#)?;
    
    println!("App: {}", config.get("app_name")?.as_string()?);
    println!("Port: {}", config.get("port")?.as_integer()?);
    
    // Method 2: Configuration management
    let mut managed_config = Config::from_string(r#"
        name = "ManagedApp"
        version = "1.0.0"
    "#)?;
    
    // Make changes
    managed_config.set("version", "1.1.0")?;
    managed_config.set("debug", true)?;
    
    // Save to file
    managed_config.save_to_file("output.noml")?;
    
    Ok(())
}
```

### Format Preservation Workflow

```rust
use noml::{parse_preserving_from_file, modify_preserving, save_preserving};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load with format preservation
    let document = parse_preserving_from_file("config.noml")?;
    
    // Make changes while preserving format
    let modified = modify_preserving(&document, |config| {
        config.set("app.version", "2.0.0")?;
        config.set("server.workers", 16)?;
        config.set("features.new_feature", true)?;
        Ok(())
    })?;
    
    // All comments, spacing, and style preserved!
    println!("Modified config:\n{}", modified);
    
    // Or save directly to file
    let mut values = document.resolve()?;
    values.set("updated_at", "2025-09-20")?;
    save_preserving(&document, &values, "config.noml")?;
    
    Ok(())
}
```

### Advanced Configuration with Builder

```rust
use noml::{Config, SchemaBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Advanced loading with defaults and validation
    let config = Config::builder()
        .default_value("debug", false)
        .default_value("port", 8080)
        .default_value("workers", 4)
        .allow_missing(true)
        .validate(true)
        .build_from_file("app.noml")?;
    
    // Schema validation
    let schema = SchemaBuilder::new()
        .require_string("app_name")
        .require_integer("port")
        .optional_bool("debug")
        .optional_integer("workers")
        .build();
    
    config.validate_schema(&schema)?;
    
    println!("Config validated successfully!");
    println!("App: {}", config.get("app_name").unwrap().as_string().unwrap());
    println!("Port: {}", config.get("port").unwrap().as_integer().unwrap());
    
    Ok(())
}
```

<br>
<hr>
<a href="#top">&uarr; <b>TOP</b></a>
<br>
<br>

---

<div align="center">
    <b>NOML Rust API Reference</b><br>
    <sub>The most advanced configuration language with revolutionary format preservation</sub>
</div>