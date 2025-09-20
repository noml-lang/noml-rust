#![allow(clippy::uninlined_format_args)]

use std::time::Instant;

fn main() {
    let toml_content = r#"
name = "test-app"
version = "1.0.0"
debug = true
numbers = [1, 2, 3, 4, 5]

[server]
host = "localhost" 
port = 8080
workers = 4
ssl = true

[database]
driver = "postgres"
host = "db.example.com"
port = 5432
name = "myapp"
pool_size = 10

[logging]
level = "info"
format = "json"
file = "/var/log/app.log"

[[workers]]
name = "worker1"
type = "background"
enabled = true

[[workers]] 
name = "worker2"
type = "api"
enabled = false
"#;

    // Warm up
    for _ in 0..100 {
        let _ = toml::from_str::<toml::Value>(toml_content).unwrap();
        let _ = noml::parse(toml_content).unwrap();
    }

    // Benchmark TOML
    let iterations = 10000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = toml::from_str::<toml::Value>(toml_content).unwrap();
    }
    let toml_time = start.elapsed();

    // Benchmark NOML
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = noml::parse(toml_content).unwrap();
    }
    let noml_time = start.elapsed();

    println!("TOML: {:?} ({:.2}µs per parse)", toml_time, toml_time.as_micros() as f64 / iterations as f64);
    println!("NOML: {:?} ({:.2}µs per parse)", noml_time, noml_time.as_micros() as f64 / iterations as f64);
    
    let ratio = toml_time.as_micros() as f64 / noml_time.as_micros() as f64;
    if ratio > 1.0 {
        println!("NOML is {:.2}x FASTER than TOML!", ratio);
    } else {
        println!("TOML is {:.2}x faster than NOML", 1.0 / ratio);
    }
}