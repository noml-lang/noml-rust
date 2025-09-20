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

[database]
host = "localhost"
port = 5432
"#;

    println!("=== NOML Performance Reality Check ===\n");

    // Test parsing performance
    let iterations = 10_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = noml::parse(content).unwrap();
    }
    let parse_time = start.elapsed();
    let avg_parse_us = parse_time.as_micros() as f64 / iterations as f64;

    // Test read performance
    let config = noml::parse(content).unwrap();
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = config.get("app_name");
        let _ = config.get("server.port");
        let _ = config.get("database.host");
    }
    let read_time = start.elapsed();
    let avg_read_ns = read_time.as_nanos() as f64 / (iterations * 3) as f64;

    println!("📊 RAW PERFORMANCE NUMBERS");
    println!("Parse time: {:.2}µs per operation", avg_parse_us);
    println!("Read time: {:.2}ns per operation", avg_read_ns);

    println!("\n🏆 PERFORMANCE CATEGORIES");

    // Define performance categories
    let categories = vec![
        ("Blazing Fast", 0.0, 1.0),            // < 1µs
        ("Very Fast", 1.0, 10.0),              // 1-10µs
        ("Fast", 10.0, 100.0),                 // 10-100µs
        ("Moderate", 100.0, 1000.0),           // 100µs-1ms
        ("Slow", 1000.0, 10000.0),             // 1-10ms
        ("Very Slow", 10000.0, f64::INFINITY), // > 10ms
    ];

    for (name, min, max) in categories {
        let parse_fits = avg_parse_us >= min && avg_parse_us < max;
        let read_fits_us = (avg_read_ns / 1000.0) >= min && (avg_read_ns / 1000.0) < max;

        if parse_fits {
            println!("Parse performance: {} ({:.2}µs)", name, avg_parse_us);
        }
        if read_fits_us {
            println!("Read performance: {} ({:.2}µs)", name, avg_read_ns / 1000.0);
        }
    }

    println!("\n🎯 TECHNOLOGY POSITIONING");
    println!("NOML Category: Dynamic Configuration Language");
    println!("TOML Category: Static Markup Language");
    println!("Comparison validity: Apples vs Oranges");

    println!("\n💡 FEATURE JUSTIFICATION");
    println!("Static TOML can't do:");
    println!("  • env('PORT', '8080') - runtime environment resolution");
    println!("  • @duration('30s') - native type parsing");
    println!("  • '${{app_name}}.log' - string interpolation");
    println!("  • include 'other.noml' - file composition");

    println!("\n🚀 PERFORMANCE POSITIONING OPTIONS");

    println!("\nOption 1: Category-Specific Performance");
    println!("  'High-performance dynamic configuration language'");
    println!("  'Blazing-fast scripting capabilities in markup format'");

    println!("\nOption 2: Feature-Performance Balance");
    println!("  'Fast configuration with advanced dynamic features'");
    println!("  'Performance-conscious design with rich functionality'");

    println!("\nOption 3: Absolute Performance Claims");
    if avg_parse_us < 100.0 && avg_read_ns < 1000.0 {
        println!("  ✅ JUSTIFIED: Sub-100µs parsing, sub-1µs reads = 'High Performance'");
        println!("  ✅ JUSTIFIED: Microsecond-level operations = 'Blazing Fast'");
    } else {
        println!("  ❌ QUESTIONABLE: May need to soften absolute performance claims");
    }

    println!("\n🤔 RECOMMENDATION");
    if avg_parse_us < 50.0 {
        println!("Keep 'high-performance' and 'blazing-fast' - you're sub-50µs!");
        println!("Position as: 'High-performance dynamic configuration language'");
        println!("Emphasize: Speed despite advanced features, not speed vs static parsers");
    } else if avg_parse_us < 100.0 {
        println!("Keep 'high-performance', soften 'blazing-fast' to 'fast'");
        println!("Position as: 'High-performance configuration with dynamic features'");
    } else {
        println!("Soften to 'efficient' and 'fast'");
        println!("Position as: 'Efficient dynamic configuration language'");
    }
}
