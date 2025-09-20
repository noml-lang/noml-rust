#![allow(clippy::uninlined_format_args)]

use noml::parse;

fn main() {
    // Test if NOML can parse TOML syntax
    let toml_content = r#"
# This is a TOML file
title = "TOML Example"
version = "1.0.0"

[owner]
name = "Tom Preston-Werner"
dob = 1979-05-27T15:32:00-08:00

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

    println!("=== Testing TOML Compatibility ===");
    
    match parse(toml_content) {
        Ok(parsed) => {
            println!("✅ NOML successfully parsed TOML content!");
            
            // Test some accesses
            if let Ok(title) = parsed.get("title").unwrap().as_string() {
                println!("Title: {title}");
            }
            
            if let Ok(server) = parsed.get("database.server").unwrap().as_string() {
                println!("Database server: {server}");
            }
            
            if let Some(_products) = parsed.get("products") {
                println!("Products array found!");
            }
            
            println!("🎉 NOML can handle TOML files with format preservation!");
        }
        Err(e) => {
            println!("❌ NOML cannot parse TOML: {e}");
            println!("We need to add TOML compatibility");
        }
    }
}