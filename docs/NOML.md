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
- **Logical Operations**: Conditional expressions and computed values
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
```

#### File Imports
Compose configurations from multiple files for modularity:

```noml
# Import shared configuration
shared_config = include("./shared.noml")
database_config = include("./database.noml")

# Override imported values
[server]
port = 8080

# Merge with imported database settings
[database]
pool_size = 20
```

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

# IP address validation
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

#### Conditional Configuration
```noml
environment = env("NODE_ENV", "development")
debug_enabled = env("DEBUG", false)

# Environment-specific database configuration
[database]
host = env("DB_HOST", "localhost")
port = env("DB_PORT", "5432")
ssl_mode = "${environment == 'production' ? 'require' : 'disable'}"

# Debug-specific logging
[logging]
level = "${debug_enabled ? 'debug' : 'info'}"
output = "${debug_enabled ? 'console' : 'file'}"
```

#### Configuration Inheritance
```noml
# Base configuration
base_config = include("./base.noml")

# Environment-specific overrides
[server]
port = env("PORT", 3000)
workers = env("WORKERS", 4)

# Merge with base database settings
[database]
# Inherits from base_config.database
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