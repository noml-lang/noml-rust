use noml::{Config, Value};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 NOML Configuration Management Demo");
    println!("====================================\n");

    // Create a config file
    let original_content = r#"# My Application Configuration
[app]
name = "MyAwesomeApp"
version = "1.0.0"
debug = false

[database]
host = "localhost"
port = 5432
name = "myapp_db"

[features]
analytics = true
caching = false
experimental = {
    ai_mode = false,
    beta_ui = true
}

languages = ["en", "es", "fr"]
"#;

    // Write the original file
    fs::write("demo_config.noml", original_content)?;
    println!("📝 Original config file created:");
    println!("{original_content}");

    // Load and modify the config
    let mut config = Config::from_file("demo_config.noml")?;
    
    println!("🔧 Making programmatic changes...");
    
    // Change some values
    config.set("app.version", Value::String("2.0.0".to_string()))?;
    config.set("app.debug", Value::Bool(true))?;
    config.set("database.port", Value::Integer(3306))?;
    config.set("features.caching", Value::Bool(true))?;
    config.set("features.experimental.ai_mode", Value::Bool(true))?;
    
    // Add a new value
    config.set("app.environment", Value::String("production".to_string()))?;
    
    // Save back to file
    config.save()?;
    
    println!("✅ Changes saved successfully!\n");
    
    // Read and display the result
    let modified_content = fs::read_to_string("demo_config.noml")?;
    println!("📄 Modified config file:");
    println!("{modified_content}");
    
    // Show what changed
    println!("🔍 What changed:");
    println!("  • app.version: 1.0.0 → 2.0.0");
    println!("  • app.debug: false → true");
    println!("  • database.port: 5432 → 3306");
    println!("  • features.caching: false → true");
    println!("  • features.experimental.ai_mode: false → true");
    println!("  • ➕ Added app.environment: production");
    
    println!("\n💡 Note: Full format preservation (comments, spacing) is a planned feature!");
    println!("   Current implementation focuses on reliable read/write of configuration values.");
    
    // Clean up
    fs::remove_file("demo_config.noml")?;
    
    println!("\n🎉 Configuration management demo complete!");
    
    Ok(())
}