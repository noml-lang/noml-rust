# NOML Examples

This directory contains comprehensive examples demonstrating all NOML features and use cases. Each example is fully documented and can be used as a starting point for your own configurations.

## Quick Start Examples

- [`basic.noml`](basic.noml) - Simple configuration showcasing core NOML syntax
- [`web_app.noml`](web_app.noml) - Web application configuration with database, caching, and logging
- [`microservices.noml`](microservices.noml) - Microservices architecture configuration
- [`development.noml`](development.noml) - Development environment setup

## Advanced Examples

- [`production.noml`](production.noml) - Production-ready configuration with monitoring and security
- [`multi_environment.noml`](multi_environment.noml) - Environment-specific configuration management
- [`cloud_native.noml`](cloud_native.noml) - Cloud-native application configuration
- [`machine_learning.noml`](machine_learning.noml) - ML pipeline configuration

## Feature-Specific Examples

- [`environment_variables.noml`](environment_variables.noml) - Environment variable usage patterns
- [`native_types.noml`](native_types.noml) - Size, duration, and URL type examples
- [`string_interpolation.noml`](string_interpolation.noml) - Variable interpolation examples
- [`file_includes.noml`](file_includes.noml) - File inclusion and modular configuration
- [`arrays_and_tables.noml`](arrays_and_tables.noml) - Complex data structures
- [`comments_and_documentation.noml`](comments_and_documentation.noml) - Documentation best practices

## Integration Examples

- [`docker.noml`](docker.noml) - Docker and containerization configuration
- [`kubernetes.noml`](kubernetes.noml) - Kubernetes deployment configuration
- [`ci_cd.noml`](ci_cd.noml) - CI/CD pipeline configuration
- [`monitoring.noml`](monitoring.noml) - Monitoring and observability setup

## Usage Patterns

Each example includes:
- **Overview**: What the example demonstrates
- **Key Features**: NOML features showcased
- **Use Cases**: When to use this pattern
- **Best Practices**: Recommended approaches
- **Related Examples**: Links to similar configurations

## Running Examples

You can parse and validate any example using the NOML CLI:

```bash
# Validate syntax
noml validate examples/basic.noml

# Parse and display structure
noml parse examples/web_app.noml
```

Or use them in your Rust code:

```rust
use noml::parse_from_file;

let config = parse_from_file("examples/basic.noml")?;
println!("App name: {}", config.get("app.name")?.as_string()?);
```

## Contributing Examples

When adding new examples:

1. Include comprehensive comments explaining each section
2. Demonstrate best practices for the use case
3. Add the example to this README with a brief description
4. Ensure the example parses successfully with `noml validate`
5. Include realistic values that would be used in practice
