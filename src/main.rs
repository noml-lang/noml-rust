//! # NOML CLI Tool
//! 
//! Command-line interface for NOML (Nested Object Markup Language).
//! This tool provides validation, conversion, and formatting capabilities.

use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("NOML v{}", noml::VERSION);
        eprintln!("Usage: {} <command> [options]", args[0]);
        eprintln!();
        eprintln!("Commands:");
        eprintln!("  validate <file>    Validate NOML syntax");
        eprintln!("  parse <file>       Parse and display structure");
        eprintln!("  version            Show version information");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  {} validate config.noml", args[0]);
        eprintln!("  {} parse app.noml", args[0]);
        process::exit(1);
    }
    
    let command = &args[1];
    
    match command.as_str() {
        "version" => {
            println!("NOML v{}", noml::VERSION);
            println!("Spec version: {}", noml::info::spec_version());
            
            let features = noml::info::features();
            println!("Features:");
            println!("  serde: {}", features.serde);
            println!("  chrono: {}", features.chrono);
        }
        
        "validate" => {
            if args.len() < 3 {
                eprintln!("Error: validate command requires a file path");
                process::exit(1);
            }
            
            let file_path = &args[2];
            validate_file(file_path);
        }
        
        "parse" => {
            if args.len() < 3 {
                eprintln!("Error: parse command requires a file path");
                process::exit(1);
            }
            
            let file_path = &args[2];
            parse_file(file_path);
        }
        
        _ => {
            eprintln!("Error: unknown command '{}'", command);
            eprintln!("Run with no arguments to see usage information.");
            process::exit(1);
        }
    }
}

fn validate_file(file_path: &str) {
    let content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading file '{}': {}", file_path, err);
            process::exit(1);
        }
    };
    
    match noml::validate(&content) {
        Ok(()) => {
            println!("✓ {} is valid NOML", file_path);
        }
        Err(err) => {
            eprintln!("✗ Validation failed for '{}':", file_path);
            eprintln!("{}", err.user_message());
            process::exit(1);
        }
    }
}

fn parse_file(file_path: &str) {
    let content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading file '{}': {}", file_path, err);
            process::exit(1);
        }
    };
    
    let document = match noml::parse(&content) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("Parse error in '{}':", file_path);
            eprintln!("{}", err.user_message());
            process::exit(1);
        }
    };
    
    let value = match document.to_value() {
        Ok(val) => val,
        Err(err) => {
            eprintln!("Conversion error in '{}':", file_path);
            eprintln!("{}", err);
            process::exit(1);
        }
    };
    
    println!("Successfully parsed '{}':", file_path);
    println!();
    println!("Structure:");
    display_value(&value, 0);
    
    // Show comments if any
    let comments = document.all_comments();
    if !comments.is_empty() {
        println!();
        println!("Comments found: {}", comments.len());
        for comment in comments {
            println!("  Line {}: {}", comment.span.start_line, comment.text);
        }
    }
}

fn display_value(value: &noml::Value, indent: usize) {
    let indent_str = "  ".repeat(indent);
    
    match value {
        noml::Value::Null => println!("{}null", indent_str),
        noml::Value::Bool(b) => println!("{}{}", indent_str, b),
        noml::Value::Integer(i) => println!("{}{}", indent_str, i),
        noml::Value::Float(f) => println!("{}{}", indent_str, f),
        noml::Value::String(s) => println!("{}\"{}\"", indent_str, s),
        noml::Value::Size(bytes) => println!("{}{}B", indent_str, bytes),
        noml::Value::Duration(secs) => println!("{}{}s", indent_str, secs),
        noml::Value::Binary(data) => println!("{}<{} bytes>", indent_str, data.len()),
        noml::Value::Array(arr) => {
            println!("{}[", indent_str);
            for (i, item) in arr.iter().enumerate() {
                print!("{}  [{}] ", indent_str, i);
                display_value(item, indent + 1);
            }
            println!("{}]", indent_str);
        }
        noml::Value::Table(table) => {
            println!("{}{{", indent_str);
            for (key, val) in table {
                print!("{}  {}: ", indent_str, key);
                display_value(val, indent + 1);
            }
            println!("{}}}", indent_str);
        }
        #[cfg(feature = "chrono")]
        noml::Value::DateTime(dt) => println!("{}{}", indent_str, dt.format("%Y-%m-%dT%H:%M:%SZ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_validate_valid_file() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, r#"
        name = "test"
        [section]
        key = "value"
        "#).unwrap();
        
        // This would normally call validate_file, but we can't test process::exit
        // Instead, test the underlying functionality
        let content = std::fs::read_to_string(file.path()).unwrap();
        assert!(noml::validate(&content).is_ok());
    }
    
    #[test]
    fn test_parse_functionality() {
        let config = r#"
        name = "test_app"
        version = 1.0
        
        [database]
        host = "localhost"
        port = 5432
        "#;
        
        let document = noml::parse(config).unwrap();
        let value = document.to_value().unwrap();
        
        assert_eq!(value.get("name").unwrap().as_string().unwrap(), "test_app");
        assert_eq!(value.get("version").unwrap().as_float().unwrap(), 1.0);
        assert_eq!(value.get("database.host").unwrap().as_string().unwrap(), "localhost");
        assert_eq!(value.get("database.port").unwrap().as_integer().unwrap(), 5432);
    }
}
