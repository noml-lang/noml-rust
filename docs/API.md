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
- **[Getting Started](#getting-started)**
  - **[Basic Parsing](#basic-parsing-usage)**
  - **[Schema Validation](#schema-validation)**

<hr>
<br>
<h2 id="installation">Installation</h2>

<br>


### Install Manually
Add this to your `Cargo.toml`:
```toml
[dependencies]
noml = "0.3.0"
```

<br>

### Install via Terminal
```bash
# Basic installation
cargo add noml
```

<br>

<hr>
<a href="#top">&uarr; <b>TOP</b></a>
<br>
<br>

<h2 id="feature-flags">Feature Flags</h2>

| Feature               | Default | Description |
|----------------------|:-------:|-------------|
| `name`  |  on     | Desc. |



<br>
<hr>
<a href="#top">&uarr; <b>TOP</b></a>
<br>
<br>


<h2 id="getting-started">Getting Started</h2>

<br>

<h2 id="basic-parsing">Basic Parsing</h2>

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

<br>

<h2 id="schema-validation">Schema Validation</h2>

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


<!-- =============================================================== -->
<br><hr><a href="#top">&uarr; <b>TOP</b></a><br><br>
<!-- =============================================================== -->

<br>