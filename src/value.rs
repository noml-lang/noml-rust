//! # NOML Runtime Value System
//! 
//! This module provides the runtime value representation for NOML data.
//! Values are the final, resolved form of NOML data after parsing and
//! processing includes, interpolations, and function calls.

use crate::error::{NomlError, Result};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::fmt;

/// A NOML value in its runtime form
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// String value
    String(String),
    /// Integer value
    Integer(i64),
    /// Floating point value
    Float(f64),
    /// Boolean value
    Bool(bool),
    /// Null value
    Null,
    /// Array of values
    Array(Vec<Value>),
    /// Table/object with key-value pairs
    Table(IndexMap<String, Value>),
    /// Native type with resolved value
    Native {
        type_name: String,
        value: Box<Value>,
        raw_args: Vec<Value>,
    },
}

impl Value {
    /// Create a new string value
    pub fn string<S: Into<String>>(s: S) -> Self {
        Value::String(s.into())
    }

    /// Create a new integer value
    pub fn integer(i: i64) -> Self {
        Value::Integer(i)
    }

    /// Create a new float value
    pub fn float(f: f64) -> Self {
        Value::Float(f)
    }

    /// Create a new boolean value
    pub fn bool(b: bool) -> Self {
        Value::Bool(b)
    }

    /// Create a null value
    pub fn null() -> Self {
        Value::Null
    }

    /// Create a new array value
    pub fn array(values: Vec<Value>) -> Self {
        Value::Array(values)
    }

    /// Create a new table value
    pub fn table(map: IndexMap<String, Value>) -> Self {
        Value::Table(map)
    }

    /// Create an empty table
    pub fn empty_table() -> Self {
        Value::Table(IndexMap::new())
    }

    /// Create a native type value
    pub fn native<S: Into<String>>(type_name: S, value: Value, raw_args: Vec<Value>) -> Self {
        Value::Native {
            type_name: type_name.into(),
            value: Box::new(value),
            raw_args,
        }
    }

    /// Get the type name of this value
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "string",
            Value::Integer(_) => "integer",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::Null => "null",
            Value::Array(_) => "array",
            Value::Table(_) => "table",
            Value::Native { .. } => "native",
        }
    }

    /// Check if this value is a string
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Check if this value is an integer
    pub fn is_integer(&self) -> bool {
        matches!(self, Value::Integer(_))
    }

    /// Check if this value is a float
    pub fn is_float(&self) -> bool {
        matches!(self, Value::Float(_))
    }

    /// Check if this value is a number (integer or float)
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Integer(_) | Value::Float(_))
    }

    /// Check if this value is a boolean
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Check if this value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Check if this value is an array
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    /// Check if this value is a table
    pub fn is_table(&self) -> bool {
        matches!(self, Value::Table(_))
    }

    /// Check if this value is a native type
    pub fn is_native(&self) -> bool {
        matches!(self, Value::Native { .. })
    }

    /// Try to get this value as a string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get this value as an integer
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get this value as a float
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Try to get this value as a boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get this value as an array
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Try to get this value as a mutable array
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Try to get this value as a table
    pub fn as_table(&self) -> Option<&IndexMap<String, Value>> {
        match self {
            Value::Table(table) => Some(table),
            _ => None,
        }
    }

    /// Try to get this value as a mutable table
    pub fn as_table_mut(&mut self) -> Option<&mut IndexMap<String, Value>> {
        match self {
            Value::Table(table) => Some(table),
            _ => None,
        }
    }

    /// Get a value from a table using a dotted path
    /// 
    /// # Example
    /// ```
    /// # use noml::value::Value;
    /// # use indexmap::IndexMap;
    /// let mut table = IndexMap::new();
    /// table.insert("database".to_string(), Value::table({
    ///     let mut db = IndexMap::new();
    ///     db.insert("host".to_string(), Value::string("localhost"));
    ///     db
    /// }));
    /// let value = Value::table(table);
    /// 
    /// assert_eq!(value.get("database.host").unwrap().as_string(), Some("localhost"));
    /// ```
    pub fn get(&self, path: &str) -> Option<&Value> {
        let mut current = self;
        
        for segment in path.split('.') {
            match current {
                Value::Table(table) => {
                    current = table.get(segment)?;
                }
                _ => return None,
            }
        }
        
        Some(current)
    }

    /// Get a mutable value from a table using a dotted path
    pub fn get_mut(&mut self, path: &str) -> Option<&mut Value> {
        let mut current = self;
        
        for segment in path.split('.') {
            match current {
                Value::Table(table) => {
                    current = table.get_mut(segment)?;
                }
                _ => return None,
            }
        }
        
        Some(current)
    }

    /// Set a value in a table using a dotted path, creating intermediate tables as needed
    pub fn set(&mut self, path: &str, value: Value) -> Result<()> {
        let segments: Vec<&str> = path.split('.').collect();
        if segments.is_empty() {
            return Err(NomlError::value("Empty path not allowed"));
        }

        // Ensure we're working with a table
        if !self.is_table() {
            return Err(NomlError::value("Cannot set path on non-table value"));
        }

        let table = self.as_table_mut().unwrap();
        let last_segment = segments.last().unwrap();
        
        // Navigate to the parent table, creating intermediate tables as needed
        let mut current_table = table;
        for segment in &segments[..segments.len() - 1] {
            let entry = current_table
                .entry(segment.to_string())
                .or_insert_with(|| Value::empty_table());
            
            if !entry.is_table() {
                return Err(NomlError::value(format!(
                    "Cannot create table at '{}': existing value is not a table",
                    segment
                )));
            }
            
            current_table = entry.as_table_mut().unwrap();
        }
        
        // Set the final value
        current_table.insert(last_segment.to_string(), value);
        Ok(())
    }

    /// Remove a value from a table using a dotted path
    pub fn remove(&mut self, path: &str) -> Option<Value> {
        let segments: Vec<&str> = path.split('.').collect();
        if segments.is_empty() {
            return None;
        }

        if !self.is_table() {
            return None;
        }

        let table = self.as_table_mut().unwrap();
        
        if segments.len() == 1 {
            return table.shift_remove(&segments[0]);
        }

        // Navigate to the parent table
        let mut current_table = table;
        for segment in &segments[..segments.len() - 1] {
            match current_table.get_mut(segment) {
                Some(Value::Table(nested_table)) => {
                    current_table = nested_table;
                }
                _ => return None,
            }
        }
        
        current_table.shift_remove(segments.last().unwrap())
    }

    /// Check if a path exists in this value
    pub fn contains(&self, path: &str) -> bool {
        self.get(path).is_some()
    }

    /// Get all keys from a table (non-recursive)
    pub fn keys(&self) -> Vec<String> {
        match self {
            Value::Table(table) => table.keys().cloned().collect(),
            _ => Vec::new(),
        }
    }

    /// Get all keys from a table recursively with dotted notation
    pub fn keys_recursive(&self) -> Vec<String> {
        let mut keys = Vec::new();
        self.collect_keys_recursive("", &mut keys);
        keys
    }

    fn collect_keys_recursive(&self, prefix: &str, keys: &mut Vec<String>) {
        match self {
            Value::Table(table) => {
                for (key, value) in table {
                    let full_key = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    
                    keys.push(full_key.clone());
                    value.collect_keys_recursive(&full_key, keys);
                }
            }
            _ => {}
        }
    }

    /// Convert this value to a JSON-like string representation
    pub fn to_json_string(&self) -> String {
        match self {
            Value::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
            Value::Integer(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(arr) => {
                let elements: Vec<String> = arr.iter().map(|v| v.to_json_string()).collect();
                format!("[{}]", elements.join(", "))
            }
            Value::Table(table) => {
                let pairs: Vec<String> = table
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, v.to_json_string()))
                    .collect();
                format!("{{{}}}", pairs.join(", "))
            }
            Value::Native { value, .. } => value.to_json_string(),
        }
    }

    /// Merge another value into this one (for tables)
    pub fn merge(&mut self, other: Value) -> Result<()> {
        match (self, other) {
            (Value::Table(ref mut table1), Value::Table(table2)) => {
                for (key, value) in table2 {
                    match table1.get_mut(&key) {
                        Some(existing) if existing.is_table() && value.is_table() => {
                            existing.merge(value)?;
                        }
                        _ => {
                            table1.insert(key, value);
                        }
                    }
                }
                Ok(())
            }
            _ => Err(NomlError::value("Can only merge tables")),
        }
    }

    /// Deep clone this value
    pub fn deep_clone(&self) -> Self {
        self.clone()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Null => write!(f, "null"),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Table(table) => {
                write!(f, "{{")?;
                for (i, (key, value)) in table.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", key, value)?;
                }
                write!(f, "}}")
            }
            Value::Native { type_name, value, .. } => {
                write!(f, "@{}({})", type_name, value)
            }
        }
    }
}

// Conversion implementations
impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Integer(i)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Integer(i as i64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<f32> for Value {
    fn from(f: f32) -> Self {
        Value::Float(f as f64)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Value::Null
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(vec: Vec<T>) -> Self {
        Value::Array(vec.into_iter().map(|item| item.into()).collect())
    }
}

impl<T: Into<Value>> From<HashMap<String, T>> for Value {
    fn from(map: HashMap<String, T>) -> Self {
        let index_map: IndexMap<String, Value> = map
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect();
        Value::Table(index_map)
    }
}

impl<T: Into<Value>> From<IndexMap<String, T>> for Value {
    fn from(map: IndexMap<String, T>) -> Self {
        let value_map: IndexMap<String, Value> = map
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect();
        Value::Table(value_map)
    }
}

/// Helper macro for creating NOML values
#[macro_export]
macro_rules! noml_value {
    // Null
    (null) => { $crate::value::Value::Null };
    
    // Boolean
    (true) => { $crate::value::Value::Bool(true) };
    (false) => { $crate::value::Value::Bool(false) };
    
    // String
    ($s:expr) => {
        match $s {
            v if v.is_string() => $crate::value::Value::String(v.to_string()),
            v => $crate::value::Value::from(v),
        }
    };
    
    // Array
    ([$($item:tt),* $(,)?]) => {
        $crate::value::Value::Array(vec![$(noml_value!($item)),*])
    };
    
    // Table
    ({$($key:expr => $value:tt),* $(,)?}) => {{
        let mut map = indexmap::IndexMap::new();
        $(map.insert($key.to_string(), noml_value!($value));)*
        $crate::value::Value::Table(map)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_creation() {
        assert_eq!(Value::string("hello"), Value::String("hello".to_string()));
        assert_eq!(Value::integer(42), Value::Integer(42));
        assert_eq!(Value::float(3.14), Value::Float(3.14));
        assert_eq!(Value::bool(true), Value::Bool(true));
        assert_eq!(Value::null(), Value::Null);
    }

    #[test]
    fn test_type_checking() {
        let string_val = Value::string("test");
        let int_val = Value::integer(42);
        let array_val = Value::array(vec![Value::integer(1), Value::integer(2)]);
        let table_val = Value::empty_table();

        assert!(string_val.is_string());
        assert!(!string_val.is_integer());
        
        assert!(int_val.is_integer());
        assert!(int_val.is_number());
        
        assert!(array_val.is_array());
        assert!(table_val.is_table());
    }

    #[test]
    fn test_table_operations() {
        let mut table = Value::empty_table();
        
        // Set nested values
        table.set("database.host", Value::string("localhost")).unwrap();
        table.set("database.port", Value::integer(5432)).unwrap();
        table.set("server.port", Value::integer(8080)).unwrap();
        
        // Get values
        assert_eq!(table.get("database.host").unwrap().as_string(), Some("localhost"));
        assert_eq!(table.get("database.port").unwrap().as_integer(), Some(5432));
        assert_eq!(table.get("server.port").unwrap().as_integer(), Some(8080));
        
        // Check contains
        assert!(table.contains("database.host"));
        assert!(table.contains("server"));
        assert!(!table.contains("nonexistent"));
        
        // Remove value
        let removed = table.remove("database.host");
        assert!(removed.is_some());
        assert!(!table.contains("database.host"));
    }

    #[test]
    fn test_macro() {
        let value = noml_value!({
            "name" => "test",
            "version" => 1,
            "features" => ["parsing", "validation"],
            "config" => {
                "debug" => true,
                "port" => 8080
            }
        });
        
        assert!(value.is_table());
        assert_eq!(value.get("name").unwrap().as_string(), Some("test"));
        assert_eq!(value.get("config.debug").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn test_conversions() {
        let string_val: Value = "hello".into();
        let int_val: Value = 42i64.into();
        let float_val: Value = 3.14f64.into();
        let bool_val: Value = true.into();
        let array_val: Value = vec![1i64, 2i64, 3i64].into();
        
        assert!(string_val.is_string());
        assert!(int_val.is_integer());
        assert!(float_val.is_float());
        assert!(bool_val.is_bool());
        assert!(array_val.is_array());
    }

    #[test]
    fn test_merge() {
        let mut table1 = noml_value!({
            "a" => 1,
            "b" => {
                "x" => 10,
                "y" => 20
            }
        });
        
        let table2 = noml_value!({
            "b" => {
                "z" => 30
            },
            "c" => 3
        });
        
        table1.merge(table2).unwrap();
        
        assert_eq!(table1.get("a").unwrap().as_integer(), Some(1));
        assert_eq!(table1.get("b.x").unwrap().as_integer(), Some(10));
        assert_eq!(table1.get("b.z").unwrap().as_integer(), Some(30));
        assert_eq!(table1.get("c").unwrap().as_integer(), Some(3));
    }
}
