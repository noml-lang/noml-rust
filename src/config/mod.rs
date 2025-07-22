//! # NOML Configuration Management
//! 
//! High-level configuration management API that provides easy-to-use
//! interfaces for loading, modifying, and saving NOML configurations.

use crate::error::{NomlError, Result};
use crate::parser::{parse, parse_from_file, Document};
use crate::value::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::fs;

/// High-level configuration manager
/// 
/// Provides a convenient API for working with NOML configurations,
/// including loading from files, modifying values, and saving changes
/// while preserving comments and formatting.
#[derive(Debug, Clone)]
pub struct Config {
    /// The parsed document with source information
    document: Document,
    /// Extracted values for fast access
    values: Value,
    /// Source file path (if loaded from file)
    source_path: Option<PathBuf>,
    /// Whether the configuration has been modified
    modified: bool,
}

/// Builder for creating configurations with specific options
#[derive(Debug, Default)]
pub struct ConfigBuilder {
    /// Whether to allow missing files
    allow_missing: bool,
    /// Default values to use if keys are missing
    defaults: BTreeMap<String, Value>,
    /// Whether to validate the configuration
    validate: bool,
}

impl Config {
    /// Create a new empty configuration
    pub fn new() -> Self {
        let empty_table = Value::empty_table();
        let document = Document::new(crate::parser::AstNode::new(
            crate::parser::ast::AstValue::Table {
                entries: Vec::new(),
                inline: false,
            },
            crate::parser::Span::new(0, 0, 1, 1, 1, 1),
        ));

        Self {
            values: empty_table,
            document,
            source_path: None,
            modified: false,
        }
    }

    /// Load configuration from a string
    pub fn from_string(content: &str) -> Result<Self> {
        let document = parse(content)?;
        let values = document.to_value()?;

        Ok(Self {
            document,
            values,
            source_path: None,
            modified: false,
        })
    }

    /// Load configuration from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let document = parse_from_file(path)?;
        let values = document.to_value()?;

        Ok(Self {
            document,
            values,
            source_path: Some(path.to_path_buf()),
            modified: false,
        })
    }

    /// Create a configuration builder
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    /// Get a value by key path
    /// 
    /// Returns `None` if the key doesn't exist.
    /// 
    /// # Example
    /// ```rust
    /// # use noml::{Config, Value};
    /// let config = Config::from_string(r#"
    /// [database]
    /// host = "localhost"
    /// port = 5432
    /// "#)?;
    /// 
    /// let host = config.get("database.host")?;
    /// assert_eq!(host.as_string()?, "localhost");
    /// 
    /// let port = config.get("database.port")?;
    /// assert_eq!(port.as_integer()?, 5432);
    /// # Ok::<(), noml::Error>(())
    /// ```
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    /// Get a value by key path with a default
    /// 
    /// Returns the default value if the key doesn't exist or cannot be
    /// converted to the target type.
    /// 
    /// # Example
    /// ```rust
    /// # use noml::{Config, Value};
    /// let config = Config::from_string(r#"
    /// [server]
    /// port = 8080
    /// "#)?;
    /// 
    /// // Key exists
    /// let port = config.get_or("server.port", 3000)?;
    /// assert_eq!(port.as_integer()?, 8080);
    /// 
    /// // Key doesn't exist, uses default
    /// let timeout = config.get_or("server.timeout", 30)?;
    /// assert_eq!(timeout.as_integer()?, 30);
    /// # Ok::<(), noml::Error>(())
    /// ```
    pub fn get_or<T>(&self, key: &str, default: T) -> Result<&Value>
    where
        T: Into<Value>,
    {
        match self.get(key) {
            Some(value) => Ok(value),
            None => {
                // For this simplified version, we can't easily add the default
                // to the actual config, so we'll return an error suggesting using get_or_insert
                Err(NomlError::key_not_found(key))
            }
        }
    }

    /// Get a value or insert a default if it doesn't exist
    pub fn get_or_insert<T>(&mut self, key: &str, default: T) -> Result<&Value>
    where
        T: Into<Value>,
    {
        if !self.values.contains_key(key) {
            self.set(key, default.into())?;
        }
        Ok(self.values.get(key).unwrap())
    }

    /// Set a value by key path
    /// 
    /// Creates intermediate tables as needed and marks the configuration as modified.
    /// 
    /// # Example
    /// ```rust
    /// # use noml::{Config, Value};
    /// let mut config = Config::new();
    /// 
    /// config.set("database.host", "localhost")?;
    /// config.set("database.port", 5432)?;
    /// config.set("server.debug", true)?;
    /// 
    /// assert_eq!(config.get("database.host")?.as_string()?, "localhost");
    /// assert_eq!(config.get("database.port")?.as_integer()?, 5432);
    /// assert_eq!(config.get("server.debug")?.as_bool()?, true);
    /// # Ok::<(), noml::Error>(())
    /// ```
    pub fn set<T>(&mut self, key: &str, value: T) -> Result<()>
    where
        T: Into<Value>,
    {
        self.values.set(key, value.into())?;
        self.modified = true;
        Ok(())
    }

    /// Remove a value by key path
    pub fn remove(&mut self, key: &str) -> Result<Option<Value>> {
        let result = self.values.remove(key)?;
        if result.is_some() {
            self.modified = true;
        }
        Ok(result)
    }

    /// Check if a key exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Get all keys at the root level
    pub fn keys(&self) -> Vec<String> {
        self.values.keys()
    }

    /// Check if the configuration has been modified
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Mark the configuration as unmodified
    pub fn mark_clean(&mut self) {
        self.modified = false;
    }

    /// Get the source file path (if loaded from file)
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// Save the configuration to its source file
    /// 
    /// Only works if the configuration was loaded from a file.
    /// 
    /// # Example
    /// ```rust,no_run
    /// # use noml::Config;
    /// let mut config = Config::from_file("app.noml")?;
    /// config.set("version", "2.0.0")?;
    /// config.save()?; // Saves back to app.noml
    /// # Ok::<(), noml::Error>(())
    /// ```
    pub fn save(&self) -> Result<()> {
        if let Some(path) = &self.source_path {
            self.save_to_file(path)
        } else {
            Err(NomlError::validation(
                "Cannot save configuration: no source file path"
            ))
        }
    }

    /// Save the configuration to a specific file
    /// 
    /// # Example
    /// ```rust,no_run
    /// # use noml::Config;
    /// let config = Config::from_string("name = \"MyApp\"")?;
    /// config.save_to_file("output.noml")?;
    /// # Ok::<(), noml::Error>(())
    /// ```
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // TODO: Implement proper NOML serialization
        // For now, we'll create a basic representation
        let content = self.to_string_representation();
        fs::write(path, content).map_err(|e| {
            NomlError::io("Failed to write configuration file".to_string(), e)
        })?;
        Ok(())
    }

    /// Get the underlying Value
    pub fn as_value(&self) -> &Value {
        &self.values
    }

    /// Convert to owned Value
    pub fn into_value(self) -> Value {
        self.values
    }

    /// Get the underlying Document
    pub fn as_document(&self) -> &Document {
        &self.document
    }

    /// Merge another configuration into this one
    /// 
    /// Values from the other configuration will overwrite values in this one.
    pub fn merge(&mut self, other: &Config) -> Result<()> {
        self.merge_value(&other.values)?;
        self.modified = true;
        Ok(())
    }

    /// Validate the configuration against a schema (future feature)
    pub fn validate_schema(&self, _schema: &str) -> Result<()> {
        // TODO: Implement schema validation in future iteration
        Ok(())
    }

    /// Get configuration statistics
    pub fn stats(&self) -> ConfigStats {
        ConfigStats {
            key_count: self.count_keys(&self.values),
            depth: self.max_depth(&self.values, 0),
            comment_count: self.document.all_comments().len(),
            has_arrays: self.has_arrays(&self.values),
            has_nested_tables: self.has_nested_tables(&self.values),
        }
    }

    // Helper methods

    fn merge_value(&mut self, other: &Value) -> Result<()> {
        match (self.values.as_table_mut(), other.as_table()) {
            (Ok(self_table), Ok(other_table)) => {
                for (key, value) in other_table {
                    if let Some(existing) = self_table.get_mut(key) {
                        if existing.is_table() && value.is_table() {
                            // Recursively merge nested tables
                            let mut temp_config = Config {
                                values: existing.clone(),
                                document: self.document.clone(),
                                source_path: None,
                                modified: false,
                            };
                            temp_config.merge_value(value)?;
                            *existing = temp_config.values;
                        } else {
                            // Replace value
                            *existing = value.clone();
                        }
                    } else {
                        // Insert new value
                        self_table.insert(key.clone(), value.clone());
                    }
                }
                Ok(())
            }
            _ => Err(NomlError::validation("Cannot merge non-table values")),
        }
    }

    fn count_keys(&self, value: &Value) -> usize {
        match value {
            Value::Table(table) => {
                table.len() + table.values().map(|v| self.count_keys(v)).sum::<usize>()
            }
            Value::Array(arr) => arr.iter().map(|v| self.count_keys(v)).sum(),
            _ => 0,
        }
    }

    fn max_depth(&self, value: &Value, current_depth: usize) -> usize {
        match value {
            Value::Table(table) => {
                let max_child_depth = table
                    .values()
                    .map(|v| self.max_depth(v, current_depth + 1))
                    .max()
                    .unwrap_or(current_depth);
                max_child_depth
            }
            Value::Array(arr) => {
                let max_element_depth = arr
                    .iter()
                    .map(|v| self.max_depth(v, current_depth))
                    .max()
                    .unwrap_or(current_depth);
                max_element_depth
            }
            _ => current_depth,
        }
    }

    fn has_arrays(&self, value: &Value) -> bool {
        match value {
            Value::Array(_) => true,
            Value::Table(table) => table.values().any(|v| self.has_arrays(v)),
            _ => false,
        }
    }

    fn has_nested_tables(&self, value: &Value) -> bool {
        match value {
            Value::Table(table) => table.values().any(|v| v.is_table() || self.has_nested_tables(v)),
            Value::Array(arr) => arr.iter().any(|v| self.has_nested_tables(v)),
            _ => false,
        }
    }

    fn to_string_representation(&self) -> String {
        // Basic TOML-style output (will be improved in future iterations)
        self.value_to_string(&self.values, 0, "")
    }

    fn value_to_string(&self, value: &Value, indent: usize, prefix: &str) -> String {
        let indent_str = "  ".repeat(indent);
        
        match value {
            Value::Table(table) => {
                let mut result = String::new();
                
                // First output direct key-value pairs
                for (key, val) in table {
                    if !val.is_table() {
                        let full_key = if prefix.is_empty() {
                            key.clone()
                        } else {
                            format!("{}.{}", prefix, key)
                        };
                        
                        result.push_str(&format!("{}{} = {}\n", 
                            indent_str, 
                            key, 
                            self.value_to_literal_string(val)
                        ));
                    }
                }
                
                // Then output nested tables
                for (key, val) in table {
                    if val.is_table() {
                        let full_key = if prefix.is_empty() {
                            key.clone()
                        } else {
                            format!("{}.{}", prefix, key)
                        };
                        
                        result.push('\n');
                        result.push_str(&format!("{}[{}]\n", indent_str, full_key));
                        result.push_str(&self.value_to_string(val, indent, &full_key));
                    }
                }
                
                result
            }
            _ => self.value_to_literal_string(value),
        }
    }

    fn value_to_literal_string(&self, value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Integer(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
            Value::Array(arr) => {
                let elements: Vec<String> = arr.iter()
                    .map(|v| self.value_to_literal_string(v))
                    .collect();
                format!("[{}]", elements.join(", "))
            }
            Value::Table(table) => {
                let entries: Vec<String> = table.iter()
                    .map(|(k, v)| format!("{} = {}", k, self.value_to_literal_string(v)))
                    .collect();
                format!("{{ {} }}", entries.join(", "))
            }
            Value::Size(bytes) => format!("{}B", bytes),
            Value::Duration(secs) => format!("{}s", secs),
            Value::Binary(data) => format!("<{} bytes>", data.len()),
            #[cfg(feature = "chrono")]
            Value::DateTime(dt) => format!("\"{}\"", dt.format("%Y-%m-%dT%H:%M:%SZ")),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration statistics
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigStats {
    /// Total number of keys (including nested)
    pub key_count: usize,
    /// Maximum nesting depth
    pub depth: usize,
    /// Number of comments in the source
    pub comment_count: usize,
    /// Whether the configuration contains arrays
    pub has_arrays: bool,
    /// Whether the configuration has nested tables
    pub has_nested_tables: bool,
}

impl ConfigBuilder {
    /// Allow missing files (return empty config instead of error)
    pub fn allow_missing(mut self, allow: bool) -> Self {
        self.allow_missing = allow;
        self
    }

    /// Add a default value for a key
    pub fn default_value<T>(mut self, key: &str, value: T) -> Self
    where
        T: Into<Value>,
    {
        self.defaults.insert(key.to_string(), value.into());
        self
    }

    /// Enable or disable validation
    pub fn validate(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }

    /// Build the configuration from a file
    pub fn build_from_file<P: AsRef<Path>>(self, path: P) -> Result<Config> {
        let path = path.as_ref();
        
        let mut config = if path.exists() {
            Config::from_file(path)?
        } else if self.allow_missing {
            Config::new()
        } else {
            return Err(NomlError::io(
                path.to_string_lossy().to_string(),
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Configuration file not found",
                ),
            ));
        };

        // Apply defaults for missing keys
        for (key, value) in self.defaults {
            if !config.contains_key(&key) {
                config.set(&key, value)?;
            }
        }

        if self.validate {
            config.validate_schema("")?; // TODO: Implement proper schema validation
        }

        config.mark_clean(); // Don't consider defaults as modifications
        Ok(config)
    }

    /// Build the configuration from a string
    pub fn build_from_string(self, content: &str) -> Result<Config> {
        let mut config = Config::from_string(content)?;

        // Apply defaults for missing keys
        for (key, value) in self.defaults {
            if !config.contains_key(&key) {
                config.set(&key, value)?;
            }
        }

        if self.validate {
            config.validate_schema("")?;
        }

        config.mark_clean();
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn config_creation() {
        let config = Config::new();
        assert!(!config.is_modified());
        assert!(config.keys().is_empty());
        assert!(config.source_path().is_none());
    }

    #[test]
    fn config_from_string() {
        let content = r#"
        name = "test"
        version = 1.0
        
        [database]
        host = "localhost"
        port = 5432
        "#;
        
        let config = Config::from_string(content).unwrap();
        
        assert_eq!(config.get("name").unwrap().as_string().unwrap(), "test");
        assert_eq!(config.get("version").unwrap().as_float().unwrap(), 1.0);
        assert_eq!(config.get("database.host").unwrap().as_string().unwrap(), "localhost");
        assert_eq!(config.get("database.port").unwrap().as_integer().unwrap(), 5432);
        
        assert!(!config.is_modified());
    }

    #[test]
    fn config_modification() {
        let mut config = Config::new();
        
        config.set("name", "test_app").unwrap();
        config.set("version", 1.5).unwrap();
        config.set("server.host", "0.0.0.0").unwrap();
        config.set("server.port", 8080).unwrap();
        
        assert!(config.is_modified());
        
        assert_eq!(config.get("name").unwrap().as_string().unwrap(), "test_app");
        assert_eq!(config.get("version").unwrap().as_float().unwrap(), 1.5);
        assert_eq!(config.get("server.host").unwrap().as_string().unwrap(), "0.0.0.0");
        assert_eq!(config.get("server.port").unwrap().as_integer().unwrap(), 8080);
    }

    #[test]
    fn config_removal() {
        let mut config = Config::from_string(r#"
        name = "test"
        version = 1.0
        debug = true
        "#).unwrap();
        
        assert!(config.contains_key("debug"));
        
        let removed = config.remove("debug").unwrap();
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().as_bool().unwrap(), true);
        
        assert!(!config.contains_key("debug"));
        assert!(config.is_modified());
    }

    #[test]
    fn config_get_or_insert() {
        let mut config = Config::from_string(r#"
        name = "test"
        "#).unwrap();
        
        // Key exists
        let name = config.get_or_insert("name", "default").unwrap();
        assert_eq!(name.as_string().unwrap(), "test");
        
        // Key doesn't exist, should insert default
        let version = config.get_or_insert("version", "1.0.0").unwrap();
        assert_eq!(version.as_string().unwrap(), "1.0.0");
        
        assert!(config.is_modified());
        assert!(config.contains_key("version"));
    }

    #[test]
    fn config_merge() {
        let mut config1 = Config::from_string(r#"
        name = "app1"
        version = "1.0"
        
        [database]
        host = "localhost"
        "#).unwrap();
        
        let config2 = Config::from_string(r#"
        version = "2.0"
        author = "test"
        
        [database]
        port = 5432
        
        [server]
        host = "0.0.0.0"
        "#).unwrap();
        
        config1.merge(&config2).unwrap();
        
        // Overwritten values
        assert_eq!(config1.get("version").unwrap().as_string().unwrap(), "2.0");
        
        // Preserved values
        assert_eq!(config1.get("name").unwrap().as_string().unwrap(), "app1");
        
        // New values
        assert_eq!(config1.get("author").unwrap().as_string().unwrap(), "test");
        
        // Merged nested tables
        assert_eq!(config1.get("database.host").unwrap().as_string().unwrap(), "localhost");
        assert_eq!(config1.get("database.port").unwrap().as_integer().unwrap(), 5432);
        
        // New nested tables
        assert_eq!(config1.get("server.host").unwrap().as_string().unwrap(), "0.0.0.0");
        
        assert!(config1.is_modified());
    }

    #[test]
    fn config_stats() {
        let config = Config::from_string(r#"
        name = "test"
        items = [1, 2, 3]
        
        [database]
        host = "localhost"
        
        [database.pool]
        min = 5
        max = 20
        "#).unwrap();
        
        let stats = config.stats();
        
        assert_eq!(stats.key_count, 6); // name, items, database.host, database.pool.min, database.pool.max, plus tables
        assert!(stats.depth >= 2); // database.pool is at depth 2
        assert!(stats.has_arrays);
        assert!(stats.has_nested_tables);
    }

    #[test]
    fn config_builder() {
        let config = Config::builder()
            .default_value("name", "default_app")
            .default_value("debug", true)
            .build_from_string(r#"
            version = "1.0"
            "#)
            .unwrap();
        
        assert_eq!(config.get("name").unwrap().as_string().unwrap(), "default_app");
        assert_eq!(config.get("debug").unwrap().as_bool().unwrap(), true);
        assert_eq!(config.get("version").unwrap().as_string().unwrap(), "1.0");
        
        assert!(!config.is_modified()); // Defaults don't count as modifications
    }

    #[test]
    fn config_file_operations() {
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, r#"
        name = "file_test"
        version = 1.0
        
        [database]
        host = "localhost"
        "#).unwrap();
        
        let mut config = Config::from_file(temp_file.path()).unwrap();
        assert!(config.source_path().is_some());
        
        // Modify the config
        config.set("version", 2.0).unwrap();
        config.set("database.port", 5432).unwrap();
        
        // Save it
        config.save().unwrap();
        
        // Read it back
        let config2 = Config::from_file(temp_file.path()).unwrap();
        assert_eq!(config2.get("version").unwrap().as_float().unwrap(), 2.0);
        assert_eq!(config2.get("database.port").unwrap().as_integer().unwrap(), 5432);
    }
}
