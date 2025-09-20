#![allow(clippy::uninlined_format_args)]

use noml::parse;

fn main() {
    println!("=== Testing Simplified TOML ===");
    
    // Test basic TOML without dates
    let simple_toml = r#"
title = "TOML Example"
version = "1.0.0"
debug = true

[owner]
name = "Tom Preston-Werner"

[database]
server = "192.168.1.100"
ports = [ 8001, 8001, 8002 ]
connection_max = 5000
enabled = true

[[products]]
name = "Hammer"
sku = 738594937

[[products]]
name = "Nail"
sku = 284758393
color = "gray"
"#;

    match parse(simple_toml) {
        Ok(parsed) => {
            println!("✅ NOML can parse basic TOML syntax!");
            
            // Test accesses
            println!("Title: {}", parsed.get("title").unwrap().as_string().unwrap());
            println!("Database server: {}", parsed.get("database.server").unwrap().as_string().unwrap());
            println!("First product: {}", parsed.get("products").unwrap().as_array().unwrap()[0].get("name").unwrap().as_string().unwrap());
            
        }
        Err(e) => {
            println!("❌ Still can't parse: {e}");
        }
    }
    
    println!("\n=== Testing TOML Date Syntax ===");
    
    // Test just the problematic date
    let date_test = r#"
dob = 1979-05-27T15:32:00-08:00
"#;
    
    match parse(date_test) {
        Ok(_) => println!("✅ TOML dates work"),
        Err(e) => println!("❌ TOML dates fail: {e}"),
    }
}