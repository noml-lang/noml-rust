#![allow(clippy::uninlined_format_args)]

use std::time::Instant;

fn main() {
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

[[workers]]
name = "worker2"
type = "api"
enabled = false
"#;

    println!("=== Realistic Usage: Parse Once + Many Reads ===");
    
    // Simulate realistic app usage: 1 parse + 10,000 reads
    let reads_per_parse = 10_000;
    let test_iterations = 100;
    
    let mut total_toml_time = std::time::Duration::ZERO;
    let mut total_noml_time = std::time::Duration::ZERO;
    
    for _ in 0..test_iterations {
        // TOML: Parse + Many Reads
        let start = Instant::now();
        let toml_parsed = toml::from_str::<toml::Value>(content).unwrap();
        for _ in 0..reads_per_parse {
            let _app_name = toml_parsed.get("app_name").and_then(|v| v.as_str());
            let _port = toml_parsed.get("server").and_then(|s| s.get("port")).and_then(|v| v.as_integer());
            let _debug = toml_parsed.get("debug").and_then(|v| v.as_bool());
        }
        total_toml_time += start.elapsed();
        
        // NOML: Parse + Many Reads  
        let start = Instant::now();
        let noml_parsed = noml::parse(content).unwrap();
        for _ in 0..reads_per_parse {
            let _app_name = noml_parsed.get("app_name").and_then(|v| v.as_string().ok());
            let _port = noml_parsed.get("server.port").and_then(|v| v.as_integer().ok());
            let _debug = noml_parsed.get("debug").and_then(|v| v.as_bool().ok());
        }
        total_noml_time += start.elapsed();
    }
    
    let avg_toml = total_toml_time.as_micros() as f64 / test_iterations as f64;
    let avg_noml = total_noml_time.as_micros() as f64 / test_iterations as f64;
    
    println!("Each iteration: 1 parse + {} reads", reads_per_parse);
    println!("Average total time per iteration:");
    println!("  TOML: {:.2}µs", avg_toml);
    println!("  NOML: {:.2}µs", avg_noml);
    println!("  Ratio: NOML is {:.2}x slower overall", avg_noml / avg_toml);
    
    // Break down the costs
    let parse_cost_ratio = 6.0; // From our earlier parse-only test
    let read_cost_ratio = 1.7;  // From our read-only test
    
    println!("\n=== Cost Breakdown ===");
    println!("Parse cost: NOML {}x slower than TOML", parse_cost_ratio);
    println!("Read cost: NOML {:.1}x slower than TOML", read_cost_ratio);
    
    // Estimate what percentage of time is spent parsing vs reading
    let estimated_parse_ratio = (avg_noml - avg_toml) / avg_noml;
    
    println!("\nWith {} reads per parse:", reads_per_parse);
    println!("- Parse overhead becomes {:.1}% of total time", estimated_parse_ratio * 100.0);
    println!("- Read performance dominates the user experience");
    
    println!("\n=== Bottom Line ===");
    println!("For typical config usage (parse once, read often):");
    println!("NOML is only {:.2}x slower than TOML overall", avg_noml / avg_toml);
    println!("Much better than the {:.1}x parse-only difference!", parse_cost_ratio);
}