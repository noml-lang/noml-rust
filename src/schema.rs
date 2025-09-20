//! # NOML Schema Validation
//!
//! Comprehensive schema validation system for NOML configurations. Provides
//! type-safe validation with detailed error reporting to catch configuration
//! errors early in development and deployment.
//!
//! ## Overview
//!
//! The schema system enables you to define expected structure and types for
//! configuration files, ensuring data integrity and providing clear error
//! messages when validation fails.
//!
//! ## Quick Start
//!
//! ```rust
//! use noml::{Config, SchemaBuilder, FieldType};
//!
//! // Define schema using builder pattern
//! let schema = SchemaBuilder::new()
//!     .require_string("app_name")
//!     .require_integer("port")
//!     .optional_bool("debug")
//!     .build();
//!
//! // Load and validate configuration
//! let config = Config::from_string(r#"
//!     app_name = "my-service"
//!     port = 8080
//!     debug = true
//! "#)?;
//!
//! config.validate_schema(&schema)?;
//!
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Advanced Schema Definition
//!
//! ```rust
//! use noml::{Schema, FieldSchema, FieldType, Value};
//! use std::collections::HashMap;
//!
//! // Create nested schema for database configuration
//! let mut db_schema = Schema::new()
//!     .required_field("host", FieldType::String)
//!     .required_field("port", FieldType::Integer)
//!     .optional_field("ssl", FieldType::Bool)
//!     .field_with_default("timeout", FieldType::Integer, Value::integer(30));
//!
//! // Main application schema
//! let app_schema = Schema::new()
//!     .required_field("name", FieldType::String)
//!     .required_field("database", FieldType::Table(db_schema))
//!     .allow_additional(false);  // Strict validation
//!
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Validation Features
//!
//! - **🔍 Type Checking** - Ensure values match expected types
//! - **📋 Required Fields** - Validate presence of mandatory configuration
//! - **🔧 Default Values** - Automatic insertion of missing optional fields
//! - **🏗️ Nested Validation** - Deep validation of table structures
//! - **📝 Descriptive Errors** - Clear messages with field paths
//! - **🔓 Flexible Schemas** - Allow or reject additional fields

use crate::error::{NomlError, Result};
use crate::value::Value;
use std::collections::HashMap;

/// Schema definition for validating NOML configurations
#[derive(Debug, Clone, PartialEq)]
pub struct Schema {
    /// Field definitions
    pub fields: HashMap<String, FieldSchema>,
    /// Whether to allow additional fields not defined in schema
    pub allow_additional: bool,
}

/// Schema definition for a field
#[derive(Debug, Clone, PartialEq)]
pub struct FieldSchema {
    /// Expected type of the field
    pub field_type: FieldType,
    /// Whether this field is required
    pub required: bool,
    /// Optional description for documentation
    pub description: Option<String>,
    /// Default value if field is missing
    pub default: Option<Value>,
}

/// Supported field types for validation
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    /// String value
    String,
    /// Integer value
    Integer,
    /// Float value
    Float,
    /// Boolean value
    Bool,
    /// Binary data
    Binary,
    /// DateTime value
    DateTime,
    /// Array of specific type
    Array(Box<FieldType>),
    /// Table/object with nested schema
    Table(Schema),
    /// Any type (no validation)
    Any,
    /// One of several types
    Union(Vec<FieldType>),
}

impl Schema {
    /// Create a new empty schema
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            allow_additional: true,
        }
    }

    /// Add a required field to the schema
    pub fn required_field(mut self, name: &str, field_type: FieldType) -> Self {
        self.fields.insert(
            name.to_string(),
            FieldSchema {
                field_type,
                required: true,
                description: None,
                default: None,
            },
        );
        self
    }

    /// Add an optional field to the schema
    pub fn optional_field(mut self, name: &str, field_type: FieldType) -> Self {
        self.fields.insert(
            name.to_string(),
            FieldSchema {
                field_type,
                required: false,
                description: None,
                default: None,
            },
        );
        self
    }

    /// Add a field with a default value
    pub fn field_with_default(mut self, name: &str, field_type: FieldType, default: Value) -> Self {
        self.fields.insert(
            name.to_string(),
            FieldSchema {
                field_type,
                required: false,
                description: None,
                default: Some(default),
            },
        );
        self
    }

    /// Set whether to allow additional fields
    pub fn allow_additional(mut self, allow: bool) -> Self {
        self.allow_additional = allow;
        self
    }

    /// Validate a value against this schema
    pub fn validate(&self, value: &Value) -> Result<()> {
        match value {
            Value::Table(table) => {
                // Check required fields
                for (field_name, field_schema) in &self.fields {
                    if field_schema.required && !table.contains_key(field_name) {
                        return Err(NomlError::validation(format!(
                            "Required field '{field_name}' is missing"
                        )));
                    }
                }

                // Validate existing fields
                for (key, val) in table {
                    if let Some(field_schema) = self.fields.get(key) {
                        self.validate_field_type(val, &field_schema.field_type, key)?;
                    } else if !self.allow_additional {
                        return Err(NomlError::validation(format!(
                            "Additional field '{key}' is not allowed"
                        )));
                    }
                }

                Ok(())
            }
            _ => Err(NomlError::validation(
                "Schema validation requires a table/object at the root".to_string(),
            )),
        }
    }

    /// Validate a field against its expected type
    fn validate_field_type(
        &self,
        value: &Value,
        expected_type: &FieldType,
        field_path: &str,
    ) -> Result<()> {
        match (value, expected_type) {
            (Value::String(_), FieldType::String) => Ok(()),
            (Value::Integer(_), FieldType::Integer) => Ok(()),
            (Value::Float(_), FieldType::Float) => Ok(()),
            (Value::Bool(_), FieldType::Bool) => Ok(()),
            (Value::Binary(_), FieldType::Binary) => Ok(()),
            #[cfg(feature = "chrono")]
            (Value::DateTime(_), FieldType::DateTime) => Ok(()),
            (_, FieldType::Any) => Ok(()),

            (Value::Array(arr), FieldType::Array(element_type)) => {
                for (i, item) in arr.iter().enumerate() {
                    let item_path = format!("{field_path}[{i}]");
                    self.validate_field_type(item, element_type, &item_path)?;
                }
                Ok(())
            }

            (Value::Table(_), FieldType::Table(nested_schema)) => nested_schema.validate(value),

            (val, FieldType::Union(types)) => {
                for field_type in types {
                    if self
                        .validate_field_type(val, field_type, field_path)
                        .is_ok()
                    {
                        return Ok(());
                    }
                }
                Err(NomlError::validation(format!(
                    "Field '{field_path}' does not match any of the expected types"
                )))
            }

            _ => Err(NomlError::validation(format!(
                "Field '{field_path}' has incorrect type. Expected {expected_type:?}, got {:?}",
                self.value_type_name(value)
            ))),
        }
    }

    /// Get a human-readable type name for a value
    fn value_type_name(&self, value: &Value) -> &'static str {
        match value {
            Value::String(_) => "String",
            Value::Integer(_) => "Integer",
            Value::Float(_) => "Float",
            Value::Bool(_) => "Bool",
            Value::Array(_) => "Array",
            Value::Table(_) => "Table",
            Value::Null => "Null",
            Value::Size(_) => "Size",
            Value::Duration(_) => "Duration",
            Value::Binary(_) => "Binary",
            #[cfg(feature = "chrono")]
            Value::DateTime(_) => "DateTime",
        }
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating schemas more easily
pub struct SchemaBuilder {
    schema: Schema,
}

impl SchemaBuilder {
    /// Create a new schema builder
    pub fn new() -> Self {
        Self {
            schema: Schema::new(),
        }
    }

    /// Add a required string field
    pub fn require_string(mut self, name: &str) -> Self {
        self.schema = self.schema.required_field(name, FieldType::String);
        self
    }

    /// Add a required integer field
    pub fn require_integer(mut self, name: &str) -> Self {
        self.schema = self.schema.required_field(name, FieldType::Integer);
        self
    }

    /// Add an optional boolean field
    pub fn optional_bool(mut self, name: &str) -> Self {
        self.schema = self.schema.optional_field(name, FieldType::Bool);
        self
    }

    /// Build the final schema
    pub fn build(self) -> Schema {
        self.schema
    }
}

impl Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;
    use std::collections::BTreeMap;

    #[test]
    fn test_basic_schema_validation() {
        let schema = Schema::new()
            .required_field("name", FieldType::String)
            .required_field("port", FieldType::Integer)
            .optional_field("debug", FieldType::Bool);

        // Valid config
        let mut config = BTreeMap::new();
        config.insert("name".to_string(), Value::String("test".to_string()));
        config.insert("port".to_string(), Value::Integer(8080));
        config.insert("debug".to_string(), Value::Bool(true));

        let valid_value = Value::Table(config);
        assert!(schema.validate(&valid_value).is_ok());

        // Missing required field
        let mut invalid_config = BTreeMap::new();
        invalid_config.insert("name".to_string(), Value::String("test".to_string()));
        // missing port

        let invalid_value = Value::Table(invalid_config);
        assert!(schema.validate(&invalid_value).is_err());
    }

    #[test]
    fn test_schema_builder() {
        let schema = SchemaBuilder::new()
            .require_string("app_name")
            .require_integer("version")
            .optional_bool("debug")
            .build();

        let mut config = BTreeMap::new();
        config.insert("app_name".to_string(), Value::String("MyApp".to_string()));
        config.insert("version".to_string(), Value::Integer(1));

        let value = Value::Table(config);
        assert!(schema.validate(&value).is_ok());
    }

    #[test]
    fn test_array_validation() {
        let schema =
            Schema::new().required_field("tags", FieldType::Array(Box::new(FieldType::String)));

        let mut config = BTreeMap::new();
        config.insert(
            "tags".to_string(),
            Value::Array(vec![
                Value::String("web".to_string()),
                Value::String("api".to_string()),
            ]),
        );

        let value = Value::Table(config);
        assert!(schema.validate(&value).is_ok());

        // Invalid array element type
        let mut invalid_config = BTreeMap::new();
        invalid_config.insert(
            "tags".to_string(),
            Value::Array(vec![
                Value::String("web".to_string()),
                Value::Integer(123), // Wrong type
            ]),
        );

        let invalid_value = Value::Table(invalid_config);
        assert!(schema.validate(&invalid_value).is_err());
    }
}
