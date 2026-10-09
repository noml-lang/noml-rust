<div id="top" align="center">
    <h1>NOML LANGUAGE</h1>
</div>

```
╔═════════════════════════════════════════╗
║  ┌───¸ ┌──┐┌──────┐┌──────────┐┌──┐     ║
║  │    ╲│  ││  ┌┐  ││          ││  │     ║ 
║  │  ╷  ╲  ││  ││  ││  ┌┐  ┌┐  ││  │     ║ 
║  │  │╲    ││  ││  ││  ││  ││  ││  │     ║ 
║  │  │ ╲   ││  ┗┘  ││  │└━━┘│  ││  ┗───┐ ║ 
║  └──┘  `──┘└──────┘└──┘    └──┘└──────┘ ║
╚═════════════════════════════════════════╝
```

**NOML** (Nested Object Markup Language) is an advanced configuration language that extends TOML with dynamic capabilities while maintaining the simplicity that makes configuration files readable and maintainable. Unlike traditional static configuration formats, NOML introduces intelligent features like variable interpolation, environment variable resolution, file imports, logical operations, and native type parsing, all designed to eliminate the boilerplate and complexity that plague modern application configuration.

The language operates under two names: **NOML** refers to the language specification itself, while **NOM** is commonly used to refer to the code, file extensions (`.noml` and `.nom`), and practical implementations. Both terms are used interchangeably throughout the ecosystem.

NOML's enhanced nesting and grouping system allows for sophisticated configuration hierarchies without sacrificing clarity. The language supports dynamic evaluation through function calls like `env()` for environment variables, `include()` for modular configuration composition, and native type constructors like `@duration()`, `@size()`, and `@url()` that provide type safety and validation at parse time. Variable interpolation enables configuration values to reference other sections, creating adaptive configurations that respond to runtime conditions.

## What Makes NOML Different

Unlike static configuration languages that force you to handle all dynamic behavior in your application code, NOML pushes intelligence into the configuration layer itself. This means your application receives fully resolved, type-safe configuration objects while the complexity of environment handling, file composition, and value interpolation happens transparently during parsing.

### Key Capabilities

- **Dynamic Resolution**: Environment variables, file imports, and value interpolation
- **Native Types**: Built-in parsing for durations, sizes, URLs, and other common types  
- **Modular Composition**: Import and merge configurations from multiple files
- **Type Safety**: Strong typing with intelligent conversion and validation
- **Source Fidelity**: Complete preservation of comments, formatting, and structure

### Design Philosophy

NOML bridges the gap between simple key-value configuration and full programming languages. It provides the dynamic capabilities you need for modern applications while maintaining the human-readable, version-control-friendly format that makes configuration management sustainable.

---

## Language Specification

*NOML Language Specification v0.6.x*

### Basic Syntax

NOML follows TOML's foundational syntax for familiarity and readability, then extends it with dynamic features.

#### Key-Value Pairs
```noml
# Basic values
app_name = "my-application"
version = "1.2.3"
debug = true
port = 8080
timeout = 30.5
```

#### Tables and Nested Structure
```noml
# Table sections
[server]
host = "localhost"
port = 8080

[database]
host = "db.example.com"
port = 5432

# Nested tables
[server.ssl]
enabled = true
cert_path = "/etc/ssl/cert.pem"
```

### Dynamic Features

#### Environment Variable Resolution
Environment variables are resolved at parse time with optional default values:

```noml
# Basic environment resolution
database_url = env("DATABASE_URL", "sqlite://local.db")
log_level = env("LOG_LEVEL", "info")

# Environment variables in nested contexts
[redis]
host = env("REDIS_HOST", "localhost")
port = env("REDIS_PORT", "6379")
password = env("REDIS_PASSWORD", "")
```

#### Variable Interpolation
Reference other configuration values using `${path.to.value}` syntax:

```noml
app_name = "my-app"
environment = "production"

# Interpolation creates dynamic values
log_file = "/var/log/${app_name}-${environment}.log"
backup_dir = "/backups/${app_name}/${environment}"

[database]
name = "${app_name}_${environment}"
connection_string = "postgres://user:pass@localhost/${database.name}"
pool_size = 20
max_overflow = ${database.pool_size}   # bare: copies the value and keeps its type
```

Rules:

- Paths start at the document root; array elements are reached by index
  (`${servers.0.host}` or `${servers[0].host}`); quote keys that contain dots
  (`${"dotted.key"}`).
- In double-quoted strings (`"..."`, `"""..."""`) the value is converted to text.
  Tables and arrays can only be copied with a bare `${path}`.
- References may point forward or chain; a cycle is an error, as is a path that
  does not exist.
- Single-quoted and raw strings (`'...'`, `'''...'''`, `r"..."`) are literal and
  never interpolated. In a double-quoted string, `$${` produces a literal `${`.
- `env()` defaults and native type arguments can use interpolation:
  `@size("${max_mb}MB")`.
- Inside an included file, paths are looked up in that file first, then in the
  file that included it.

#### File Imports
Compose configurations from multiple files for modularity:

```noml
# Import shared configuration; the file's contents become the value of the key
shared_config = include("./shared.noml")
database_config = include "./database.noml"   # parentheses are optional

[server]
port = 8080

[database]
pool_size = 20
# Values from an included file can be referenced like any other value
timeout = ${database_config.timeout}
```

Relative paths are resolved from the directory of the file that contains the
`include`. Include cycles and includes nested more than 10 levels deep are errors.

#### Dates and Times
TOML date-time literals are written bare and read as date/time values:

```noml
created  = 1979-05-27T07:32:00Z         # offset date-time
deadline = 1979-05-27 07:32:00-08:00    # a space may replace the T
local    = 1979-05-27T07:32:00.999      # local date-time
birthday = 1979-05-27                   # local date
alarm    = 07:32:00                     # local time
```

Invalid dates such as `2023-02-29` are parse errors. In Rust they are
`Value::DateTime(noml::Datetime)`; with the `chrono` feature an offset date-time converts
to a `chrono` date-time.

#### Native Type Constructors
Parse and validate common types at configuration time:

```noml
# Duration parsing with validation
request_timeout = @duration("30s")
session_lifetime = @duration("24h")
cleanup_interval = @duration("5m")

# Size parsing with unit conversion  
max_file_size = @size("10MB")
memory_limit = @size("2GB")
cache_size = @size("512KB")

# URL validation and parsing
api_endpoint = @url("https://api.example.com/v1")
webhook_url = @url("http://localhost:3000/webhook")

# IP address validation (IPv4, IPv6, optional CIDR prefix)
allowed_hosts = [@ip("192.168.1.1"), @ip("10.0.0.0/8")]
```

### Collections and Complex Types

#### Arrays
```noml
# Simple arrays
ports = [8080, 8081, 8082]
environments = ["dev", "staging", "prod"]

# Mixed type arrays with native types
timeouts = [@duration("1s"), @duration("5s"), @duration("30s")]
allowed_sizes = [@size("1MB"), @size("10MB"), @size("100MB")]
```

#### Inline Tables
```noml
# Compact table syntax
database = { host = "localhost", port = 5432, ssl = true }
redis = { host = env("REDIS_HOST", "localhost"), port = 6379 }

# Mixed with native types
limits = { 
    timeout = @duration("30s"), 
    size = @size("10MB"),
    connections = 100 
}
```

### Advanced Patterns

#### Configuration Inheritance
```noml
# Base configuration
base_config = include("./base.noml")

# Environment-specific overrides
[server]
port = env("PORT", 3000)
workers = env("WORKERS", 4)

# Reuse values from the base file
[database]
host = ${base_config.database.host}
timeout = @duration("10s")
pool_size = env("DB_POOL_SIZE", 10)
```

#### Template-Style Configuration
```noml
# Configuration templates
service_name = "user-service"
version = "1.0.0"
namespace = env("K8S_NAMESPACE", "default")

# Template expansion
[kubernetes]
deployment_name = "${service_name}-${version}"
image = "registry.example.com/${service_name}:${version}"
namespace = "${namespace}"

[monitoring]
metrics_endpoint = "/metrics"
health_check = "/health"
service_url = "http://${service_name}.${namespace}.svc.cluster.local"
```

### Comments and Documentation

NOML preserves comments and formatting, making it ideal for documented configuration:

```noml
# Application Configuration
# This file contains the main configuration for the application.
# Environment variables are used for deployment-specific values.

# Core Application Settings
app_name = "my-application"
version = "1.2.3"                    # Semantic versioning
debug = env("DEBUG", false)          # Enable debug mode via DEBUG env var

# Server Configuration
[server]
host = "0.0.0.0"                     # Bind to all interfaces
port = env("PORT", 8080)             # Configurable port
request_timeout = @duration("30s")    # 30 second request timeout

# Database Connection
[database]
# Primary database connection
url = env("DATABASE_URL", "postgresql://localhost/myapp")
pool_size = env("DB_POOL_SIZE", 10)  # Connection pool size
timeout = @duration("5s")            # Query timeout

# Redis Cache Configuration  
[cache]
enabled = env("CACHE_ENABLED", true)
url = env("REDIS_URL", "redis://localhost:6379")
ttl = @duration("1h")                # Default cache TTL
```

This specification covers NOML v0.6.x features and syntax. The language continues to evolve while maintaining backward compatibility and the core principle of making configuration both powerful and maintainable.