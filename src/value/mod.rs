//! # NOML Value System
//!
//! Comprehensive value types and operations for NOML data representation.
//! This module provides the core [`Value`] enum and all associated functionality
//! for working with configuration data in a type-safe manner.
//!
//! ## Value Types
//!
//! NOML supports all standard configuration data types plus specialized types
//! for common configuration patterns:
//!
//! - **Primitives**: `null`, `bool`, `i64`, `f64`, `String`
//! - **Collections**: `Array<Value>`, `Table<String, Value>`
//! - **Native Types**: `Size`, `Duration`, `Binary`
//! - **Optional**: `DateTime` (with `chrono` feature)
//!
//! ## Type Conversions
//!
//! The [`Value`] type provides safe conversions with comprehensive error handling:
//!
//! ```rust
//! use noml::Value;
//!
//! let value = Value::string("42");
//!
//! // Safe conversions with error handling
//! let as_int = value.as_integer()?;  // Ok(42)
//! let as_float = value.as_float()?;  // Ok(42.0)
//! let as_bool = value.as_bool();     // Err(type mismatch)
//!
//! # Ok::<(), noml::error::NomlError>(())
//! ```
//!
//! ## Smart String Conversions
//!
//! String values support intelligent parsing for booleans and numbers:
//!
//! ```rust
//! use noml::Value;
//!
//! // Boolean parsing
//! assert_eq!(Value::string("true").as_bool()?, true);
//! assert_eq!(Value::string("yes").as_bool()?, true);
//! assert_eq!(Value::string("1").as_bool()?, true);
//! assert_eq!(Value::string("false").as_bool()?, false);
//!
//! // Numeric parsing
//! assert_eq!(Value::string("123").as_integer()?, 123);
//! assert_eq!(Value::string("3.14").as_float()?, 3.14);
//!
//! # Ok::<(), noml::error::NomlError>(())
//! ```
//!
//! ## Native Type Support
//!
//! NOML includes specialized types for common configuration values:
//!
//! ```rust
//! use noml::Value;
//!
//! // File sizes with automatic parsing
//! let size = Value::size(1024 * 1024);  // 1MB in bytes
//!
//! // Time durations in seconds
//! let timeout = Value::duration(30.0);  // 30 seconds
//!
//! // Binary data
//! let data = Value::Binary(vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]);
//! ```
//!
//! ## Path-Based Access
//!
//! Access nested values using dot-notation paths:
//!
//! ```rust
//! use noml::Value;
//! use std::collections::BTreeMap;
//!
//! let mut config = BTreeMap::new();
//! let mut server = BTreeMap::new();
//! server.insert("host".to_string(), Value::string("localhost"));
//! server.insert("port".to_string(), Value::integer(8080));
//! config.insert("server".to_string(), Value::table(server));
//!
//! let root = Value::table(config);
//!
//! // Access nested values
//! let host = root.get("server.host").unwrap();
//! let port = root.get("server.port").unwrap();
//!
//! assert_eq!(host.as_string()?, "localhost");
//! assert_eq!(port.as_integer()?, 8080);
//!
//! # Ok::<(), noml::error::NomlError>(())
//! ```

use crate::error::{NomlError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};

/// A NOML value - the fundamental unit of data in NOML documents.
///
/// Values are designed to be lightweight, cloneable, and convertible
/// to/from Rust native types with zero-copy operations where possible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// Null/empty value
    Null,

    /// Boolean value (true/false)
    Bool(bool),

    /// Integer value (64-bit signed)
    Integer(i64),

    /// Floating-point value (64-bit)
    Float(f64),

    /// String value (UTF-8)
    String(String),

    /// Array of values
    Array(Vec<Value>),

    /// Table/object with string keys
    Table(BTreeMap<String, Value>),

    /// Native date/time value (optional feature)
    #[cfg(feature = "chrono")]
    #[serde(with = "chrono::serde::ts_seconds")]
    DateTime(DateTime<Utc>),

    /// Raw binary data
    Binary(Vec<u8>),

    /// Size value (bytes, with human-readable format)
    Size(u64),

    /// Duration value (seconds, with human-readable format)
    Duration(f64),
}

impl Value {
    /// Create a new null value
    pub fn null() -> Self {
        Value::Null
    }

    /// Create a new boolean value
    pub fn bool(value: bool) -> Self {
        Value::Bool(value)
    }

    /// Create a new integer value
    pub fn integer(value: i64) -> Self {
        Value::Integer(value)
    }

    /// Create a new float value
    pub fn float(value: f64) -> Self {
        Value::Float(value)
    }

    /// Create a new string value
    pub fn string(value: impl Into<String>) -> Self {
        Value::String(value.into())
    }

    /// Create a new array value
    pub fn array(values: Vec<Value>) -> Self {
        Value::Array(values)
    }

    /// Create a new table value
    pub fn table(map: BTreeMap<String, Value>) -> Self {
        Value::Table(map)
    }

    /// Create an empty table
    pub fn empty_table() -> Self {
        Value::Table(BTreeMap::new())
    }

    /// Create a new size value from bytes
    pub fn size(bytes: u64) -> Self {
        Value::Size(bytes)
    }

    /// Create a new duration value from seconds
    pub fn duration(seconds: f64) -> Self {
        Value::Duration(seconds)
    }

    /// Get the type name of this value
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Integer(_) => "integer",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Table(_) => "table",
            #[cfg(feature = "chrono")]
            Value::DateTime(_) => "datetime",
            Value::Binary(_) => "binary",
            Value::Size(_) => "size",
            Value::Duration(_) => "duration",
        }
    }

    /// Check if this value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Check if this value is a boolean
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Check if this value is a number (integer or float)
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Integer(_) | Value::Float(_))
    }

    /// Check if this value is a string
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Check if this value is an array
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    /// Check if this value is a table
    pub fn is_table(&self) -> bool {
        matches!(self, Value::Table(_))
    }

    /// Convert value to boolean with intelligent string parsing
    ///
    /// Performs type conversion to boolean with support for common
    /// string representations. Non-zero integers are treated as true.
    ///
    /// # String Parsing
    ///
    /// The following string values are recognized (case-insensitive):
    /// - **True**: `"true"`, `"yes"`, `"1"`, `"on"`
    /// - **False**: `"false"`, `"no"`, `"0"`, `"off"`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use noml::Value;
    ///
    /// // Direct boolean values
    /// assert_eq!(Value::Bool(true).as_bool()?, true);
    /// assert_eq!(Value::Bool(false).as_bool()?, false);
    ///
    /// // String parsing
    /// assert_eq!(Value::string("true").as_bool()?, true);
    /// assert_eq!(Value::string("YES").as_bool()?, true);
    /// assert_eq!(Value::string("1").as_bool()?, true);
    /// assert_eq!(Value::string("false").as_bool()?, false);
    /// assert_eq!(Value::string("no").as_bool()?, false);
    ///
    /// // Integer conversion
    /// assert_eq!(Value::integer(1).as_bool()?, true);
    /// assert_eq!(Value::integer(0).as_bool()?, false);
    /// assert_eq!(Value::integer(-5).as_bool()?, true);
    ///
    /// # Ok::<(), noml::error::NomlError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`NomlError`] for:
    /// - Unrecognized string values
    /// - Incompatible types (arrays, tables, etc.)
    pub fn as_bool(&self) -> Result<bool> {
        match self {
            Value::Bool(b) => Ok(*b),
            Value::String(s) => match s.to_lowercase().as_str() {
                "true" | "yes" | "1" | "on" => Ok(true),
                "false" | "no" | "0" | "off" => Ok(false),
                _ => Err(NomlError::type_error(s, "boolean", self.type_name())),
            },
            Value::Integer(i) => Ok(*i != 0),
            _ => Err(NomlError::type_error(
                self.to_string(),
                "boolean",
                self.type_name(),
            )),
        }
    }

    /// Try to convert to integer
    pub fn as_integer(&self) -> Result<i64> {
        match self {
            Value::Integer(i) => Ok(*i),
            Value::Float(f) => {
                if f.fract() == 0.0 && *f >= i64::MIN as f64 && *f <= i64::MAX as f64 {
                    Ok(*f as i64)
                } else {
                    Err(NomlError::type_error(f.to_string(), "integer", "float"))
                }
            }
            Value::String(s) => s
                .parse::<i64>()
                .map_err(|_| NomlError::type_error(s, "integer", "string")),
            Value::Bool(b) => Ok(if *b { 1 } else { 0 }),
            _ => Err(NomlError::type_error(
                self.to_string(),
                "integer",
                self.type_name(),
            )),
        }
    }

    /// Try to convert to float
    pub fn as_float(&self) -> Result<f64> {
        match self {
            Value::Float(f) => Ok(*f),
            Value::Integer(i) => Ok(*i as f64),
            Value::String(s) => s
                .parse::<f64>()
                .map_err(|_| NomlError::type_error(s, "float", "string")),
            _ => Err(NomlError::type_error(
                self.to_string(),
                "float",
                self.type_name(),
            )),
        }
    }

    /// Try to convert to string
    pub fn as_string(&self) -> Result<&str> {
        match self {
            Value::String(s) => Ok(s),
            _ => Err(NomlError::type_error(
                self.to_string(),
                "string",
                self.type_name(),
            )),
        }
    }

    /// Convert to owned string
    pub fn into_string(self) -> Result<String> {
        match self {
            Value::String(s) => Ok(s),
            _ => Ok(self.to_string()),
        }
    }

    /// Try to get as array reference
    pub fn as_array(&self) -> Result<&Vec<Value>> {
        match self {
            Value::Array(arr) => Ok(arr),
            _ => Err(NomlError::type_error(
                self.to_string(),
                "array",
                self.type_name(),
            )),
        }
    }

    /// Try to get as mutable array reference
    pub fn as_array_mut(&mut self) -> Result<&mut Vec<Value>> {
        match self {
            Value::Array(arr) => Ok(arr),
            _ => Err(NomlError::type_error(
                self.to_string(),
                "array",
                self.type_name(),
            )),
        }
    }

    /// Try to get as table reference
    pub fn as_table(&self) -> Result<&BTreeMap<String, Value>> {
        match self {
            Value::Table(table) => Ok(table),
            _ => Err(NomlError::type_error(
                self.to_string(),
                "table",
                self.type_name(),
            )),
        }
    }

    /// Try to get as mutable table reference
    pub fn as_table_mut(&mut self) -> Result<&mut BTreeMap<String, Value>> {
        match self {
            Value::Table(table) => Ok(table),
            _ => Err(NomlError::type_error(
                self.to_string(),
                "table",
                self.type_name(),
            )),
        }
    }

    /// Get a value by key path (dot-separated)
    ///
    /// Examples:
    /// - `get("key")` - get direct key
    /// - `get("section.key")` - get nested key
    /// - `get("array.0")` - get array index
    pub fn get(&self, path: &str) -> Option<&Value> {
        let mut current = self;

        for segment in path.split('.') {
            match current {
                Value::Table(table) => {
                    current = table.get(segment)?;
                }
                Value::Array(array) => {
                    let index = segment.parse::<usize>().ok()?;
                    current = array.get(index)?;
                }
                _ => return None,
            }
        }

        Some(current)
    }

    /// Set a value by key path (dot-separated)
    /// Creates intermediate tables as needed
    pub fn set(&mut self, path: &str, value: Value) -> Result<()> {
        let segments: Vec<&str> = path.split('.').collect();
        if segments.is_empty() {
            return Err(NomlError::validation("Empty path"));
        }

        // Ensure we're working with a table at the root
        if !self.is_table() {
            *self = Value::empty_table();
        }

        let mut current = self;

        // Navigate to the parent of the final key
        for segment in &segments[..segments.len() - 1] {
            let table = current.as_table_mut()?;

            // Create intermediate table if it doesn't exist
            if !table.contains_key(*segment) {
                table.insert(segment.to_string(), Value::empty_table());
            }

            current = table.get_mut(*segment).unwrap();

            // Ensure intermediate values are tables
            if !current.is_table() {
                return Err(NomlError::validation(format!(
                    "Cannot set nested key: '{segment}' is not a table"
                )));
            }
        }

        // Set the final value
        let final_key = segments.last().unwrap();
        let table = current.as_table_mut()?;
        table.insert(final_key.to_string(), value);

        Ok(())
    }

    /// Remove a value by key path
    pub fn remove(&mut self, path: &str) -> Result<Option<Value>> {
        let segments: Vec<&str> = path.split('.').collect();
        if segments.is_empty() {
            return Err(NomlError::validation("Empty path"));
        }

        if segments.len() == 1 {
            // Direct key removal
            let table = self.as_table_mut()?;
            return Ok(table.remove(segments[0]));
        }

        // Navigate to parent
        let mut current = self;
        for segment in &segments[..segments.len() - 1] {
            current = match current.as_table_mut()?.get_mut(*segment) {
                Some(value) => value,
                None => return Ok(None),
            };
        }

        // Remove from parent
        let final_key = segments.last().unwrap();
        let table = current.as_table_mut()?;
        Ok(table.remove(*final_key))
    }

    /// Check if a key path exists
    pub fn contains_key(&self, path: &str) -> bool {
        self.get(path).is_some()
    }

    /// Get all keys at the current level (non-recursive)
    pub fn keys(&self) -> Vec<String> {
        match self {
            Value::Table(table) => table.keys().cloned().collect(),
            _ => Vec::new(),
        }
    }

    /// Get the length of arrays or tables
    pub fn len(&self) -> usize {
        match self {
            Value::Array(arr) => arr.len(),
            Value::Table(table) => table.len(),
            Value::String(s) => s.len(),
            _ => 0,
        }
    }

    /// Check if the value is empty (for collections)
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Integer(i) => write!(f, "{i}"),
            Value::Float(fl) => write!(f, "{fl}"),
            Value::String(s) => write!(f, "\"{s}\""),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            Value::Table(table) => {
                write!(f, "{{")?;
                for (i, (key, value)) in table.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{key}: {value}")?;
                }
                write!(f, "}}")
            }
            #[cfg(feature = "chrono")]
            Value::DateTime(dt) => write!(f, "{}", dt.format("%Y-%m-%dT%H:%M:%SZ")),
            Value::Binary(data) => write!(f, "<{} bytes>", data.len()),
            Value::Size(bytes) => write!(f, "{}", format_size(*bytes)),
            Value::Duration(seconds) => write!(f, "{}", format_duration(*seconds)),
        }
    }
}

/// Format a size in bytes to human-readable format
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
    if bytes == 0 {
        return "0B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{bytes}B")
    } else {
        format!("{:.1}{}", size, UNITS[unit_index])
    }
}

/// Format a duration in seconds to human-readable format
fn format_duration(seconds: f64) -> String {
    if seconds < 1.0 {
        format!("{}ms", (seconds * 1000.0) as u64)
    } else if seconds < 60.0 {
        format!("{seconds:.1}s")
    } else if seconds < 3600.0 {
        format!("{:.1}m", seconds / 60.0)
    } else if seconds < 86400.0 {
        format!("{:.1}h", seconds / 3600.0)
    } else {
        format!("{:.1}d", seconds / 86400.0)
    }
}

// Implement From traits for easy value creation
impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Integer(i as i64)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Integer(i)
    }
}

impl From<f32> for Value {
    fn from(f: f32) -> Self {
        Value::Float(f as f64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<Vec<Value>> for Value {
    fn from(arr: Vec<Value>) -> Self {
        Value::Array(arr)
    }
}

impl From<BTreeMap<String, Value>> for Value {
    fn from(table: BTreeMap<String, Value>) -> Self {
        Value::Table(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_creation_and_type_checking() {
        let null_val = Value::null();
        assert!(null_val.is_null());
        assert_eq!(null_val.type_name(), "null");

        let bool_val = Value::bool(true);
        assert!(bool_val.is_bool());
        assert!(bool_val.as_bool().unwrap());

        let int_val = Value::integer(42);
        assert!(int_val.is_number());
        assert_eq!(int_val.as_integer().unwrap(), 42);

        let str_val = Value::string("hello");
        assert!(str_val.is_string());
        assert_eq!(str_val.as_string().unwrap(), "hello");
    }

    #[test]
    fn nested_key_operations() {
        let mut value = Value::empty_table();

        // Set nested values
        value
            .set("server.host", Value::string("localhost"))
            .unwrap();
        value.set("server.port", Value::integer(8080)).unwrap();
        value
            .set("database.url", Value::string("postgres://..."))
            .unwrap();

        // Get nested values
        assert_eq!(
            value.get("server.host").unwrap().as_string().unwrap(),
            "localhost"
        );
        assert_eq!(
            value.get("server.port").unwrap().as_integer().unwrap(),
            8080
        );

        // Check existence
        assert!(value.contains_key("server.host"));
        assert!(value.contains_key("database.url"));
        assert!(!value.contains_key("nonexistent.key"));

        // Remove values
        assert!(value.remove("server.host").unwrap().is_some());
        assert!(!value.contains_key("server.host"));
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn type_conversions() {
        // String to bool
        let true_val = Value::string("true");
        assert!(true_val.as_bool().unwrap());

        let false_val = Value::string("false");
        assert!(!false_val.as_bool().unwrap());

        // Float to int (exact)
        let float_val = Value::float(42.0);
        assert_eq!(float_val.as_integer().unwrap(), 42);

        // String to number
        let str_int = Value::string("123");
        assert_eq!(str_int.as_integer().unwrap(), 123);

        let str_float = Value::string("3.14");
        assert_eq!(str_float.as_float().unwrap(), 3.14);
    }

    #[test]
    fn size_and_duration_formatting() {
        let size_val = Value::size(1536); // 1.5KB
        assert_eq!(size_val.to_string(), "1.5KB");

        let duration_val = Value::duration(90.0); // 1.5 minutes
        assert_eq!(duration_val.to_string(), "1.5m");
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn from_trait_implementations() {
        let _: Value = true.into();
        let _: Value = 42i32.into();
        let _: Value = 42i64.into();
        let _: Value = 3.14f32.into();
        let _: Value = 3.14f64.into();
        let _: Value = "hello".into();
        let _: Value = String::from("world").into();
        let _: Value = vec![Value::integer(1), Value::integer(2)].into();
    }
}
