#!/usr/bin/env rust-script
//! # NOML Async Usage Example
//!
//! This example shows how to use NOML's async features.
//! Run with: cargo run --example async_demo --features async

use noml::Config;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 NOML Async Demo");
    println!("==================\n");

    // Set some environment variables for the demo
    env::set_var("PORT", "8080");
    env::set_var("DATABASE_URL", "postgresql://localhost/myapp");

    // Create a sample config file
    let config_content = r#"
# Sample async configuration
app_name = "AsyncApp"
version = "1.0.0"

[server]
host = "0.0.0.0"
port = env("PORT", "3000")
workers = 4

[database]
url = env("DATABASE_URL", "sqlite:memory:")
pool_size = 10
"#;

    // Write config file asynchronously
    tokio::fs::write("demo_config.noml", config_content).await?;
    println!("✅ Config file written async");

    // Load config asynchronously - non-blocking!
    let config = Config::load_async("demo_config.noml").await?;
    println!("✅ Config loaded async");

    // Access values safely
    let app_name = config.get("app_name").unwrap().as_string().unwrap();
    let port = config.get("server.port").unwrap().as_string().unwrap();
    let db_url = config.get("database.url").unwrap().as_string().unwrap();

    println!("� Configuration:");
    println!("   App: {app_name}");
    println!("   Port: {port}");
    println!("   Database: {db_url}");

    // Modify config
    let mut config = config;
    config.set("last_started", "2025-09-19T12:00:00Z")?;

    // Save asynchronously
    config.save_async("demo_config.noml").await?;
    println!("✅ Config saved async");

    // Reload to verify changes
    config.reload_async().await?;
    println!("✅ Config reloaded async");

    let last_started = config.get("last_started").unwrap().as_string().unwrap();
    println!("   Last started: {last_started}");

    // Clean up
    tokio::fs::remove_file("demo_config.noml").await?;
    println!("🗑️  Cleaned up");

    println!("\n🎉 Async demo completed!");
    println!("NOML is ready for modern async Rust applications! 🚀");

    Ok(())
}
