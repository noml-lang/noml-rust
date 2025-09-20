//! # NOML Macros
//!
//! Convenient macros for creating NOML values programmatically.

/// Create a NOML value using a convenient macro syntax
///
/// This macro allows you to create complex `Value` structures using a JSON-like syntax
/// that's easy to read and write.
///
/// # Example
///
/// ```rust
/// use noml::{noml_value, Value};
///
/// let config = noml_value!({
///     "name" => "my-app",
///     "version" => "1.0.0",
///     "features" => ["parsing", "validation"],
///     "server" => {
///         "host" => "localhost",
///         "port" => 8080
///     },
///     "debug" => true,
///     "timeout" => 30.5
/// });
///
/// assert_eq!(config.get("server.port").unwrap().as_integer().unwrap(), 8080);
/// assert_eq!(config.get("features").unwrap().as_array().unwrap().len(), 2);
/// ```
///
/// # Supported Types
///
/// - **Strings**: `"text"` → `Value::String`
/// - **Integers**: `42` → `Value::Integer`  
/// - **Floats**: `3.14` → `Value::Float`
/// - **Booleans**: `true`, `false` → `Value::Bool`
/// - **Arrays**: `[1, 2, 3]` → `Value::Array`
/// - **Tables**: `{ "key" => "value" }` → `Value::Table`
/// - **Null**: `null` → `Value::Null`
#[macro_export]
macro_rules! noml_value {
    // Null value
    (null) => {
        $crate::Value::Null
    };

    // Boolean values
    (true) => {
        $crate::Value::Bool(true)
    };
    (false) => {
        $crate::Value::Bool(false)
    };

    // Array values
    ([ $($item:tt),* $(,)? ]) => {
        $crate::Value::Array(vec![
            $(noml_value!($item)),*
        ])
    };

    // Table values
    ({ $($key:tt => $value:tt),* $(,)? }) => {{
        let mut table = std::collections::BTreeMap::new();
        $(
            let key_value = noml_value!($key);
            let key_str = match key_value {
                $crate::Value::String(s) => s,
                _ => panic!("Table keys must be strings"),
            };
            table.insert(key_str, noml_value!($value));
        )*
        $crate::Value::Table(table)
    }};

    // String literal
    ($val:literal) => {
        $crate::Value::from($val)
    };
}

#[cfg(test)]
mod tests {
    use crate::Value;

    #[test]
    fn test_noml_value_primitives() {
        // Test basic types
        assert_eq!(noml_value!(null), Value::Null);
        assert_eq!(noml_value!(true), Value::Bool(true));
        assert_eq!(noml_value!(false), Value::Bool(false));
        assert_eq!(noml_value!("hello"), Value::String("hello".to_string()));
        assert_eq!(noml_value!(42), Value::Integer(42));
        assert_eq!(noml_value!(3.15), Value::Float(3.15));
    }

    #[test]
    fn test_noml_value_array() {
        let arr = noml_value!([1, 2, 3]);
        if let Value::Array(values) = arr {
            assert_eq!(values.len(), 3);
            assert_eq!(values[0], Value::Integer(1));
            assert_eq!(values[1], Value::Integer(2));
            assert_eq!(values[2], Value::Integer(3));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_noml_value_mixed_array() {
        let arr = noml_value!([1, "hello", true, null]);
        if let Value::Array(values) = arr {
            assert_eq!(values.len(), 4);
            assert_eq!(values[0], Value::Integer(1));
            assert_eq!(values[1], Value::String("hello".to_string()));
            assert_eq!(values[2], Value::Bool(true));
            assert_eq!(values[3], Value::Null);
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_noml_value_table() {
        let table = noml_value!({
            "name" => "test",
            "port" => 8080,
            "debug" => true
        });

        if let Value::Table(map) = table {
            assert_eq!(map.len(), 3);
            assert_eq!(map.get("name").unwrap(), &Value::String("test".to_string()));
            assert_eq!(map.get("port").unwrap(), &Value::Integer(8080));
            assert_eq!(map.get("debug").unwrap(), &Value::Bool(true));
        } else {
            panic!("Expected table");
        }
    }

    #[test]
    fn test_noml_value_nested() {
        let config = noml_value!({
            "app" => {
                "name" => "my-app",
                "version" => "1.0.0"
            },
            "features" => ["parsing", "validation"],
            "server" => {
                "host" => "localhost",
                "port" => 8080
            }
        });

        // Test accessing nested values
        assert_eq!(
            config.get("app.name").unwrap().as_string().unwrap(),
            "my-app"
        );
        assert_eq!(
            config.get("server.port").unwrap().as_integer().unwrap(),
            8080
        );

        let features = config.get("features").unwrap().as_array().unwrap();
        assert_eq!(features.len(), 2);
        assert_eq!(features[0].as_string().unwrap(), "parsing");
    }
}
