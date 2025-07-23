//! # NOML Resolver
//! 
//! This module handles the resolution of dynamic NOML features:
//! - Environment variable lookups via env() function
//! - File inclusion via include statements  
//! - Variable interpolation via ${path} syntax
//! - Native type resolution via @type() syntax

use crate::error::{NomlError, Result};
use crate::parser::ast::{AstNode, AstValue, Document, Span, Key, TableEntry, Comments, StringStyle};
use crate::parser::parse_file;
use crate::value::Value;
use indexmap::IndexMap;
use std::collections::{HashMap, BTreeMap};
use std::env;
use std::path::{Path, PathBuf};

/// Configuration for the resolver
#[derive(Debug, Clone)]
pub struct ResolverConfig {
    /// Base path for resolving relative includes
    pub base_path: Option<PathBuf>,
    /// Environment variables to use (if None, uses std::env)
    pub env_vars: Option<HashMap<String, String>>,
    /// Maximum include depth to prevent infinite recursion
    pub max_include_depth: usize,
    /// Whether to allow missing environment variables
    pub allow_missing_env: bool,
    /// Custom native type resolvers
    pub native_resolvers: HashMap<String, NativeResolver>,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        let mut native_resolvers = HashMap::new();
        
        // Register built-in native types
        native_resolvers.insert("size".to_string(), NativeResolver::new(resolve_size));
        native_resolvers.insert("duration".to_string(), NativeResolver::new(resolve_duration));
        native_resolvers.insert("regex".to_string(), NativeResolver::new(resolve_regex));
        native_resolvers.insert("url".to_string(), NativeResolver::new(resolve_url));
        
        Self {
            base_path: None,
            env_vars: None,
            max_include_depth: 10,
            allow_missing_env: false,
            native_resolvers,
        }
    }
}

/// A native type resolver function
pub struct NativeResolver {
    resolver: Box<dyn Fn(&[Value]) -> Result<Value> + Send + Sync>,
}

impl Clone for NativeResolver {
    fn clone(&self) -> Self {
        // NativeResolver is not trivially clonable, but for built-ins this is fine.
        // For custom resolvers, users must ensure they are clonable.
        // Here we panic if someone tries to clone a custom resolver.
        panic!("NativeResolver cannot be cloned. Use only built-in resolvers or implement Clone manually if needed.");
    }
}

impl NativeResolver {
    /// Creates a new `NativeResolver` from the given resolver function.
    pub fn new<F>(resolver: F) -> Self 
    where
        F: Fn(&[Value]) -> Result<Value> + Send + Sync + 'static,
    {
        Self {
            resolver: Box::new(resolver),
        }
    }
    
    /// Resolves the native type using the provided arguments.
    pub fn resolve(&self, args: &[Value]) -> Result<Value> {
        (self.resolver)(args)
    }
}

impl std::fmt::Debug for NativeResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NativeResolver")
    }
}

/// The main resolver for NOML documents
pub struct Resolver {
    config: ResolverConfig,
    include_stack: Vec<PathBuf>,
    variables: IndexMap<String, Value>,
}

impl Resolver {
    /// Create a new resolver with default configuration
    pub fn new() -> Self {
        Self::with_config(ResolverConfig::default())
    }
    
    /// Create a new resolver with custom configuration
    pub fn with_config(config: ResolverConfig) -> Self {
        Self {
            config,
            include_stack: Vec::new(),
            variables: IndexMap::new(),
        }
    }
    
    /// Set the base path for resolving relative includes
    pub fn with_base_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.config.base_path = Some(path.into());
        self
    }
    
    /// Set custom environment variables
    pub fn with_env_vars(mut self, env_vars: HashMap<String, String>) -> Self {
        self.config.env_vars = Some(env_vars);
        self
    }
    
    /// Add a custom native type resolver
    pub fn with_native_resolver<S: Into<String>>(mut self, name: S, resolver: NativeResolver) -> Self {
        self.config.native_resolvers.insert(name.into(), resolver);
        self
    }
    
    /// Resolve a document, processing all includes, interpolations, and function calls
    pub fn resolve(&mut self, document: Document) -> Result<Value> {
        // Start with an empty variable context
        self.variables.clear();
        self.include_stack.clear();
        
        // Resolve the root node
        let resolved = self.resolve_node(&document.root)?;
        
        // Extract the final value
        self.extract_value(resolved)
    }
    
    /// Resolve a single AST node
    fn resolve_node(&mut self, node: &AstNode) -> Result<AstNode> {
        match &node.value {
            AstValue::String { value, style, has_escapes } => {
                // Check for interpolation in strings
                let resolved_value = self.resolve_interpolation_in_string(value)?;
                Ok(AstNode::new(
                    AstValue::String {
                        value: resolved_value,
                        style: style.clone(),
                        has_escapes: *has_escapes,
                    },
                    node.span.clone(),
                ))
            }
            
            AstValue::Array { elements, multiline, trailing_comma } => {
                let mut resolved_elements = Vec::new();
                for element in elements {
                    resolved_elements.push(self.resolve_node(element)?);
                }
                Ok(AstNode::new(
                    AstValue::Array {
                        elements: resolved_elements,
                        multiline: *multiline,
                        trailing_comma: *trailing_comma,
                    },
                    node.span.clone(),
                ))
            }
            
            AstValue::Table { entries, inline } => {
                let mut resolved_entries = Vec::new();
                for entry in entries {
                    let resolved_value = self.resolve_node(&entry.value)?;
                    resolved_entries.push(TableEntry {
                        key: entry.key.clone(),
                        value: resolved_value,
                        comments: entry.comments.clone(),
                    });
                }
                Ok(AstNode::new(
                    AstValue::Table {
                        entries: resolved_entries,
                        inline: *inline,
                    },
                    node.span.clone(),
                ))
            }

            AstValue::FunctionCall { name, args } => {
                match name.as_str() {
                    "env" => self.resolve_env_function(args, &node.span),
                    _ => Err(NomlError::parse(
                        format!("Unknown function: {}", name),
                        node.span.start,
                        0,
                    )),
                }
            }

            AstValue::Native { type_name, args } => {
                self.resolve_native_type(type_name, args, &node.span)
            }

            AstValue::Interpolation { path } => {
                let value = self.resolve_variable_path(path)?;
                Ok(AstNode::new(value, node.span.clone()))
            }

            AstValue::Include { path } => {
                self.resolve_include(path, &node.span)
            }

            // Pass through literal values unchanged
            _ => Ok(node.clone()),
        }
    }

    fn resolve_env_function(&self, args: &[AstNode], span: &Span) -> Result<AstNode> {
        if args.is_empty() || args.len() > 2 {
            return Err(NomlError::parse(
                "env() requires 1 or 2 arguments".to_string(),
                span.start,
                0,
            ));
        }

        // Get the environment variable name
        let var_name = match &args[0].value {
            AstValue::String { value, .. } => value,
            _ => return Err(NomlError::parse(
                "env() first argument must be a string".to_string(),
                span.start,
                0,
            )),
        };

        // Get the optional default value
        let default_value = if args.len() == 2 {
            Some(self.extract_value(args[1].clone())?)
        } else {
            None
        };

        // Look up the environment variable
        let env_value = if let Some(ref env_vars) = self.config.env_vars {
            env_vars.get(var_name).cloned()
        } else {
            env::var(var_name).ok()
        };

        let result_value = if let Some(val) = env_value {
            Value::String(val)
        } else if let Some(default) = default_value {
            default
        } else if self.config.allow_missing_env {
            Value::Null
        } else {
            return Err(NomlError::parse(format!(
                "Environment variable '{}' not found and no default provided",
                var_name
            ), span.start, 0));
        };

        // Convert back to AST node
        Ok(self.value_to_ast_node(result_value, span.clone()))
    }

    fn resolve_native_type(&self, type_name: &str, args: &[AstNode], span: &Span) -> Result<AstNode> {
        // Convert args to values
        let arg_values: Result<Vec<Value>> = args.iter()
            .map(|arg| self.extract_value(arg.clone()))
            .collect();
        let arg_values = arg_values?;

        // Look up the resolver
        let resolver = self.config.native_resolvers.get(type_name)
            .ok_or_else(|| NomlError::parse(format!("Unknown native type: @{}", type_name), span.start, 0))?;

        // Resolve the native type
        resolver.resolve(&arg_values)?;

        // Create a native value node
        let native_value = AstValue::Native {
            type_name: type_name.to_string(),
            args: args.to_vec(),
        };

        Ok(AstNode::new(native_value, span.clone()))
    }

    fn resolve_variable_path(&self, path: &str) -> Result<AstValue> {
        // This is a placeholder - in a real implementation, you'd maintain
        // a variable scope and resolve paths within it
        Err(NomlError::parse(format!(
            "Variable interpolation not yet implemented: {}",
            path
        ), 0, 0))
    }

    /// Resolve include statements
    fn resolve_include(&mut self, include_path: &str, span: &Span) -> Result<AstNode> {
        // Check include depth
        if self.include_stack.len() >= self.config.max_include_depth {
            return Err(NomlError::parse(
                format!(
                    "Maximum include depth ({}) exceeded",
                    self.config.max_include_depth
                ),
                span.start,
                0,
            ));
        }

        let resolved_path = self.resolve_include_path(include_path)?;

        // Check for circular includes
        if self.include_stack.contains(&resolved_path) {
            return Err(NomlError::parse(
                format!("Circular include detected: {:?}", resolved_path),
                span.start,
                0,
            ));
        }

        // Parse the included file
        self.include_stack.push(resolved_path.clone());
        let included_doc = parse_file(&resolved_path)
            .map_err(|e| NomlError::parse(format!("Failed to parse include '{}': {}", resolved_path.display(), e), span.start, 0))?;

        // Resolve the included document
        let resolved_include = self.resolve_node(&included_doc.root)?;
        self.include_stack.pop();

        Ok(resolved_include)
    }
    
    /// Resolve an include path relative to the current file or base path
    fn resolve_include_path(&self, include_path: &str) -> Result<PathBuf> {
        let path = Path::new(include_path);
        
        if path.is_absolute() {
            Ok(path.to_path_buf())
        } else {
            // Try to resolve relative to current file or base path
            let base = if let Some(current_file) = self.include_stack.last() {
                current_file.parent().unwrap_or(Path::new("."))
            } else if let Some(ref base_path) = self.config.base_path {
                base_path.as_path()
            } else {
                Path::new(".")
            };
            
            Ok(base.join(path))
        }
    }
    
    /// Resolve interpolation patterns in strings like "Hello ${name}!"
    fn resolve_interpolation_in_string(&self, text: &str) -> Result<String> {
        let mut result = String::new();
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$' && chars.peek() == Some(&'{') {
                chars.next(); // consume '{'

                // Read until '}'
                let mut var_path = String::new();
                let mut found_close = false;
                while let Some(&next_ch) = chars.peek() {
                    if next_ch == '}' {
                        chars.next(); // consume '}'
                        found_close = true;
                        break;
                    } else {
                        var_path.push(next_ch);
                        chars.next();
                    }
                }

                if !found_close {
                    return Err(NomlError::parse("Unclosed interpolation in string".to_string(), 0, 0));
                }

                // Resolve the variable path
                let value = self.resolve_variable_path(&var_path)?;
                let resolved_value = self.extract_value(AstNode::new(value, Span::default()))?;
                result.push_str(&resolved_value.to_string());
            } else {
                result.push(ch);
            }
        }

        Ok(result)
    }
    
    /// Extract a runtime Value from an AST node
    fn extract_value(&self, node: AstNode) -> Result<Value> {
        match node.value {
            AstValue::String { value, .. } => Ok(Value::String(value)),
            AstValue::Integer { value, .. } => Ok(Value::Integer(value)),
            AstValue::Float { value, .. } => Ok(Value::Float(value)),
            AstValue::Bool(value) => Ok(Value::Bool(value)),
            AstValue::Null => Ok(Value::Null),
            AstValue::Table { entries, .. } => {
                let mut table = BTreeMap::new();
                for entry in entries {
                    let key = entry.key.to_string();
                    let value = self.extract_value(entry.value)?;
                    table.insert(key, value);
                }
                Ok(Value::Table(table))
            }
            AstValue::Array { elements, .. } => {
                let mut arr = Vec::new();
                for element in elements {
                    arr.push(self.extract_value(element.clone())?);
                }
                Ok(Value::Array(arr))
            }
            AstValue::Native { type_name, args } => {
                // Convert args to values
                let arg_values: Result<Vec<Value>> = args.iter()
                    .map(|arg| self.extract_value(arg.clone()))
                    .collect();
                let arg_values = arg_values?;

                // Resolve the native type
                let resolver = self.config.native_resolvers.get(&type_name)
                    .ok_or_else(|| NomlError::parse(format!("Unknown native type: @{}", type_name), 0, 0))?;

                let resolved_value = resolver.resolve(&arg_values)?;

                // No Value::Native variant exists, so just return the resolved value
                Ok(resolved_value)
            }
            _ => Err(NomlError::parse("Cannot extract value from unresolved AST node".to_string(), 0, 0)),
        }
    }

    /// Convert a runtime Value back to an AST node
    fn value_to_ast_node(&self, value: Value, span: Span) -> AstNode {
        let ast_value = match value {
            Value::String(s) => AstValue::String {
                value: s,
                style: StringStyle::Double,
                has_escapes: false,
            },
            Value::Integer(i) => AstValue::Integer {
                value: i,
                raw: i.to_string(),
            },
            Value::Float(f) => AstValue::Float {
                value: f,
                raw: f.to_string(),
            },
            Value::Bool(b) => AstValue::Bool(b),
            Value::Null => AstValue::Null,
            Value::Array(arr) => {
                let elements = arr.into_iter()
                    .map(|v| self.value_to_ast_node(v, span.clone()))
                    .collect();
                AstValue::Array {
                    elements,
                    multiline: false,
                    trailing_comma: false,
                }
            }
            Value::Table(table) => {
                let entries = table.into_iter()
                    .map(|(k, v)| TableEntry {
                        key: Key::simple(k, span.clone()),
                        value: self.value_to_ast_node(v, span.clone()),
                        comments: Comments::new(),
                    })
                    .collect();
                AstValue::Table {
                    entries,
                    inline: false,
                }
            }
            Value::Binary(_) | Value::Size(_) | Value::Duration(_) => {
                // Not implemented for AST conversion
                unimplemented!("Conversion from Value::{:?} to AstValue is not implemented", value)
            }
        };

        AstNode::new(ast_value, span)
    }
} // <-- Close impl Resolver

// Built-in native type resolvers

fn resolve_size(args: &[Value]) -> Result<Value> {
    if args.len() != 1 {
        return Err(NomlError::parse("@size() requires exactly 1 argument".to_string(), 0, 0));
    }
    let size_str = match args[0].as_string() {
        Ok(s) => s,
        Err(e) => return Err(NomlError::parse(format!("@size() argument must be a string: {}", e), 0, 0)),
    };
    match parse_size(size_str) {
        Some(n) => Ok(Value::Integer(n)),
        None => Err(NomlError::parse(format!("Invalid size format: {}", size_str), 0, 0)),
    }
}

/// Parse size strings like "10MB", "1.5GB", etc.
fn parse_size(size_str: &str) -> Option<i64> {
    let size_str = size_str.trim().to_uppercase();
    let (number_part, unit_part) = if let Some(pos) = size_str.find(|c: char| !char::is_numeric(c) && c != '.') {
        (&size_str[..pos], &size_str[pos..])
    } else {
        (size_str.as_str(), "")
    };

    let number: f64 = number_part.parse().ok()?;

    let multiplier = match unit_part.trim() {
        "" => 1,
        "B" => 1,
        "KB" => 1_024,
        "MB" => 1_024 * 1_024,
        "GB" => 1_024 * 1_024 * 1_024,
        "TB" => 1_024_i64.pow(4),
        "PB" => 1_024_i64.pow(5),
        _ => return None,
    };

    Some((number * multiplier as f64) as i64)
}

fn resolve_duration(args: &[Value]) -> Result<Value> {
    if args.len() != 1 {
        return Err(NomlError::parse("@duration() requires exactly 1 argument".to_string(), 0, 0));
    }
    let duration_str = match args[0].as_string() {
        Ok(s) => s,
        Err(e) => return Err(NomlError::parse(format!("@duration() argument must be a string: {}", e), 0, 0)),
    };
    match parse_duration(duration_str) {
        Some(n) => Ok(Value::Float(n)),
        None => Err(NomlError::parse(format!("Invalid duration format: {}", duration_str), 0, 0)),
    }
}

/// Parse duration strings like "30s", "5m", "2h", etc.
fn parse_duration(duration_str: &str) -> Option<f64> {
    let duration_str = duration_str.trim().to_lowercase();

    let (number_part, unit_part) = if let Some(pos) = duration_str.find(|c: char| c.is_alphabetic()) {
        (&duration_str[..pos], &duration_str[pos..])
    } else {
        (duration_str.as_str(), "s")
    };

    let number: f64 = number_part.parse().ok()?;

    let multiplier = match unit_part {
        "ns" => 1e-9,
        "us" | "μs" => 1e-6,
        "ms" => 1e-3,
        "" | "s" => 1.0,
        "m" | "min" => 60.0,
        "h" | "hr" | "hour" => 3600.0,
        "d" | "day" => 86400.0,
        "w" | "week" => 604800.0,
        _ => return None,
    };

    Some(number * multiplier)
}

fn resolve_regex(args: &[Value]) -> Result<Value> {
    if args.len() != 1 {
        return Err(NomlError::parse("@regex() requires exactly 1 argument".to_string(), 0, 0));
    }
    let regex_str = match args[0].as_string() {
        Ok(s) => s,
        Err(e) => return Err(NomlError::parse(format!("@regex() argument must be a string: {}", e), 0, 0)),
    };

    // Validate the regex (in a real implementation, you'd use the regex crate)
    // For now, just return the string
    Ok(Value::String(regex_str.to_string()))
}

fn resolve_url(args: &[Value]) -> Result<Value> {
    if args.len() != 1 {
        return Err(NomlError::parse("@url() requires exactly 1 argument".to_string(), 0, 0));
    }
    let url_str = match args[0].as_string() {
        Ok(s) => s,
        Err(e) => return Err(NomlError::parse(format!("@url() argument must be a string: {}", e), 0, 0)),
    };

    // Basic URL validation (in a real implementation, you'd use the url crate)
    if url_str.starts_with("http://") || url_str.starts_with("https://") {
        Ok(Value::String(url_str.to_string()))
    } else {
        Err(NomlError::parse(format!("Invalid URL format: {}", url_str), 0, 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("1KB"), Some(1024));
        assert_eq!(parse_size("1MB"), Some(1024 * 1024));
        assert_eq!(parse_size("1.5GB"), Some((1.5 * 1024.0 * 1024.0 * 1024.0) as i64));
        assert_eq!(parse_size("invalid"), None);
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s"), Some(30.0));
        assert_eq!(parse_duration("5m"), Some(300.0));
        assert_eq!(parse_duration("2h"), Some(7200.0));
        assert_eq!(parse_duration("1d"), Some(86400.0));
        assert_eq!(parse_duration("invalid"), None);
    }

    #[test]
    fn test_resolve_size_duration_url() {
        let size_result = resolve_size(&[Value::String("10MB".to_string())]).unwrap();
        assert_eq!(size_result.as_integer().unwrap(), 10 * 1024 * 1024);

        let duration_result = resolve_duration(&[Value::String("30s".to_string())]).unwrap();
        assert_eq!(duration_result.as_float().unwrap(), 30.0);

        let url_result = resolve_url(&[Value::String("https://example.com".to_string())]).unwrap();
        assert_eq!(url_result.as_string().unwrap(), "https://example.com");

        let url_result = resolve_url(&[Value::String("https://example.com".to_string())]).unwrap();
        assert_eq!(url_result.as_string().unwrap(), "https://example.com");
    }
}