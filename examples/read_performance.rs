#![allow(clippy::uninlined_format_args)]

use std::time::Instant;
use std::collections::HashMap;

fn main() {
    // Test content for reading performance
    let content = r#"
app_name = "MyApp"
version = "2.1.0"
debug = false

[server]
host = "0.0.0.0"
port = 8080
timeout = 30
ssl = true

[database]
host = "localhost"
port = 5432
name = "mydb"
user = "admin"
password = "secret"
pool_size = 10

[[workers]]
name = "worker1"
type = "background"
enabled = true
memory = 512

[[workers]]
name = "worker2"
type = "api"
enabled = false
memory = 1024

[[workers]]
name = "worker3"
type = "queue"
enabled = true
memory = 256
"#;

    let iterations = 100_000;
    
    println!("=== Config Read Performance Comparison ===");
    println!("Testing {} read operations per parsed config\n", iterations);
    
    // Parse once, read many times - TOML
    let toml_parsed = toml::from_str::<toml::Value>(content).unwrap();
    
    let start = Instant::now();
    for _ in 0..iterations {
        // Typical config reads
        let _app_name = toml_parsed.get("app_name").and_then(|v| v.as_str());
        let _version = toml_parsed.get("version").and_then(|v| v.as_str()); 
        let _debug = toml_parsed.get("debug").and_then(|v| v.as_bool());
        let _host = toml_parsed.get("server").and_then(|s| s.get("host")).and_then(|v| v.as_str());
        let _port = toml_parsed.get("server").and_then(|s| s.get("port")).and_then(|v| v.as_integer());
        let _ssl = toml_parsed.get("server").and_then(|s| s.get("ssl")).and_then(|v| v.as_bool());
        let _db_host = toml_parsed.get("database").and_then(|d| d.get("host")).and_then(|v| v.as_str());
        let _db_port = toml_parsed.get("database").and_then(|d| d.get("port")).and_then(|v| v.as_integer());
        let _workers = toml_parsed.get("workers").and_then(|w| w.as_array());
    }
    let toml_read_time = start.elapsed();
    
    // Parse once, read many times - NOML
    let noml_parsed = noml::parse(content).unwrap();
    
    let start = Instant::now();
    for _ in 0..iterations {
        // Typical config reads using NOML API
        let _app_name = noml_parsed.get("app_name").and_then(|v| v.as_string().ok());
        let _version = noml_parsed.get("version").and_then(|v| v.as_string().ok());
        let _debug = noml_parsed.get("debug").and_then(|v| v.as_bool().ok());
        let _host = noml_parsed.get("server.host").and_then(|v| v.as_string().ok());
        let _port = noml_parsed.get("server.port").and_then(|v| v.as_integer().ok());
        let _ssl = noml_parsed.get("server.ssl").and_then(|v| v.as_bool().ok());
        let _db_host = noml_parsed.get("database.host").and_then(|v| v.as_string().ok());
        let _db_port = noml_parsed.get("database.port").and_then(|v| v.as_integer().ok());
        let _workers = noml_parsed.get("workers");
    }
    let noml_read_time = start.elapsed();
    
    // Also test HashMap baseline for comparison
    let mut hashmap: HashMap<String, String> = HashMap::new();
    hashmap.insert("app_name".to_string(), "MyApp".to_string());
    hashmap.insert("version".to_string(), "2.1.0".to_string());
    hashmap.insert("debug".to_string(), "false".to_string());
    hashmap.insert("server.host".to_string(), "0.0.0.0".to_string());
    hashmap.insert("server.port".to_string(), "8080".to_string());
    hashmap.insert("server.ssl".to_string(), "true".to_string());
    hashmap.insert("database.host".to_string(), "localhost".to_string());
    hashmap.insert("database.port".to_string(), "5432".to_string());
    
    let start = Instant::now();
    for _ in 0..iterations {
        let _app_name = hashmap.get("app_name");
        let _version = hashmap.get("version");
        let _debug = hashmap.get("debug");
        let _host = hashmap.get("server.host");
        let _port = hashmap.get("server.port");
        let _ssl = hashmap.get("server.ssl");
        let _db_host = hashmap.get("database.host");
        let _db_port = hashmap.get("database.port");
        let _workers = hashmap.get("workers");
    }
    let hashmap_read_time = start.elapsed();
    
    println!("Results (average per read operation):");
    println!("  HashMap baseline:  {:.2}ns per read", hashmap_read_time.as_nanos() as f64 / iterations as f64);
    println!("  TOML reads:        {:.2}ns per read", toml_read_time.as_nanos() as f64 / iterations as f64);
    println!("  NOML reads:        {:.2}ns per read", noml_read_time.as_nanos() as f64 / iterations as f64);
    
    let toml_vs_hashmap = toml_read_time.as_nanos() as f64 / hashmap_read_time.as_nanos() as f64;
    let noml_vs_hashmap = noml_read_time.as_nanos() as f64 / hashmap_read_time.as_nanos() as f64;
    let noml_vs_toml = noml_read_time.as_nanos() as f64 / toml_read_time.as_nanos() as f64;
    
    println!("\nRelative Performance:");
    println!("  TOML is {:.2}x slower than HashMap", toml_vs_hashmap);
    println!("  NOML is {:.2}x slower than HashMap", noml_vs_hashmap);
    println!("  NOML is {:.2}x slower than TOML", noml_vs_toml);
    
    println!("\n=== Analysis ===");
    println!("This measures the cost of value access after parsing.");
    println!("NOML provides path-based access (\"server.port\") vs manual navigation.");
    println!("Both are much slower than raw HashMap due to type checking and navigation.");
}