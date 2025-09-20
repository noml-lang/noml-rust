#![allow(clippy::uninlined_format_args)]

use std::time::Instant;

fn main() {
    // Complex NOML content that TOML can't handle
    let noml_content = r#"
# Application Configuration with dynamic features
app_name = "MyApp"
version = "2.1.0"
debug = env("DEBUG", "false")
secret_key = env("SECRET_KEY", "default-secret")

# Server with native types and env vars
[server]
host = env("SERVER_HOST", "0.0.0.0")
port = env("SERVER_PORT", "8080") 
timeout = @duration("30s")
max_request_size = @size("10MB")
ssl_cert = env("SSL_CERT_PATH")

# Include external config
database = include "database.noml"

# String interpolation
[logging]
log_file = "/var/log/${app_name}.log"
format = "json"
level = env("LOG_LEVEL", "info")

# Array of tables with complex values
[[workers]]
name = "worker1"
type = "background"
memory_limit = @size("512MB")
timeout = @duration("5m")
enabled = env("WORKER1_ENABLED", "true")

[[workers]]
name = "worker2" 
type = "api"
memory_limit = @size("1GB")
timeout = @duration("30s")
enabled = env("WORKER2_ENABLED", "false")
"#;

    // Simple TOML content for comparison
    let simple_toml = r#"
app_name = "MyApp"
version = "2.1.0"
debug = false

[server]
host = "0.0.0.0"
port = 8080
ssl = true

[[workers]]
name = "worker1"
type = "background"
enabled = true

[[workers]]
name = "worker2"
type = "api"
enabled = false
"#;

    println!("=== NOML Advanced Features Test ===");
    
    let iterations = 1000;
    
    // Test NOML with advanced features (TOML can't do this)
    let start = Instant::now();
    for _ in 0..iterations {
        let _config = noml::parse(noml_content);
        // Note: This will have some errors due to missing env vars and includes,
        // but we're measuring parsing performance, not resolution
    }
    let noml_advanced_time = start.elapsed();
    
    // Test simple parsing comparison
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = toml::from_str::<toml::Value>(simple_toml).unwrap();
    }
    let toml_simple_time = start.elapsed();
    
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = noml::parse(simple_toml).unwrap();
    }
    let noml_simple_time = start.elapsed();

    println!("Simple TOML content:");
    println!("  TOML: {:.2}µs per parse", toml_simple_time.as_micros() as f64 / iterations as f64);
    println!("  NOML: {:.2}µs per parse", noml_simple_time.as_micros() as f64 / iterations as f64);
    println!("  Ratio: TOML is {:.2}x faster", noml_simple_time.as_micros() as f64 / toml_simple_time.as_micros() as f64);
    
    println!("\nAdvanced NOML content (TOML cannot parse this):");
    println!("  NOML: {:.2}µs per parse", noml_advanced_time.as_micros() as f64 / iterations as f64);
    
    println!("\n=== Analysis ===");
    println!("NOML provides advanced features that TOML cannot:");
    println!("- Environment variable resolution: env(\"VAR\", \"default\")");
    println!("- Native types: @duration(\"30s\"), @size(\"10MB\")");
    println!("- File includes: include \"other.noml\"");
    println!("- String interpolation: \"/var/log/${{app_name}}.log\"");
    println!("- Full source fidelity preservation for tooling");
    println!("- Comment and formatting preservation");
    
    let overhead = (noml_simple_time.as_micros() as f64 / toml_simple_time.as_micros() as f64) - 1.0;
    println!("\nNOML overhead for advanced capabilities: {:.1}%", overhead * 100.0);
}