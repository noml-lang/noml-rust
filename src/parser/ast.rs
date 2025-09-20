//! # Abstract Syntax Tree
//! 
//! AST representation of parsed NOML documents with full fidelity preservation.
//! This includes comments, formatting, and source location information for
//! perfect round-trip serialization and error reporting.

use crate::error::{NomlError, Result};
use crate::value::Value;
use std::collections::BTreeMap;
use std::fmt;

/// Represents a complete NOML document with metadata
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// Root value of the document (typically a table)
    pub root: AstNode,
    /// Source file path (if loaded from file)
    pub source_path: Option<String>,
    /// Original source text (for error reporting and round-trip)
    pub source_text: Option<String>,
}

/// An AST node with source location and metadata
#[derive(Debug, Clone, PartialEq)]
pub struct AstNode {
    /// The actual value
    pub value: AstValue,
    /// Source location information
    pub span: Span,
    /// Comments associated with this node
    pub comments: Comments,
}

/// Source location information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// Starting byte offset in source
    pub start: usize,
    /// Ending byte offset in source
    pub end: usize,
    /// Starting line number (1-indexed)
    pub start_line: usize,
    /// Starting column number (1-indexed) 
    pub start_column: usize,
    /// Ending line number (1-indexed)
    pub end_line: usize,
    /// Ending column number (1-indexed)
    pub end_column: usize,
}

impl Default for Span {
    fn default() -> Self {
        Span {
            start: 0,
            end: 0,
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 1,
        }
    }
}

/// Comments associated with an AST node
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Comments {
    /// Comments before this node
    pub before: Vec<Comment>,
    /// Inline comment on the same line
    pub inline: Option<Comment>,
    /// Comments after this node
    pub after: Vec<Comment>,
}

/// A single comment with location
#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    /// Comment text (without # prefix)
    pub text: String,
    /// Source location
    pub span: Span,
    /// Comment style
    pub style: CommentStyle,
}

/// Different comment styles
#[derive(Debug, Clone, PartialEq)]
pub enum CommentStyle {
    /// Line comment starting with #
    Line,
    /// Multi-line comment (future extension)
    Block,
}

/// AST value types - like Value but with source preservation
#[derive(Debug, Clone, PartialEq)]
pub enum AstValue {
    /// Null value
    Null,
    
    /// Boolean value
    Bool(bool),
    
    /// Integer value with original text representation
    Integer {
        /// The integer value
        value: i64,
        /// Original text (for preserving formatting like 0x123, 0o777)
        raw: String,
    },
    
    /// Float value with original text representation
    Float {
        /// The float value
        value: f64,
        /// Original text (for preserving precision and format)
        raw: String,
    },
    
    /// String value with quote style preservation
    String {
        /// The string value
        value: String,
        /// Original quote style (single, double, triple, etc.)
        style: StringStyle,
        /// Whether this string contained escape sequences
        has_escapes: bool,
    },
    
    /// Array with formatting preservation
    Array {
        /// Elements of the array
        elements: Vec<AstNode>,
        /// Whether array is formatted on multiple lines
        multiline: bool,
        /// Trailing comma
        trailing_comma: bool,
    },
    
    /// Table with key ordering and formatting preservation
    Table {
        /// Ordered key-value pairs
        entries: Vec<TableEntry>,
        /// Whether table uses inline format { key = value }
        inline: bool,
    },
    
    /// Function call (for env(), size(), etc.)
    FunctionCall {
        /// Function name
        name: String,
        /// Arguments
        args: Vec<AstNode>,
    },
    
    /// Variable interpolation ${path}
    Interpolation {
        /// The path to interpolate
        path: String,
    },
    
    /// Include/import statement
    Include {
        /// Path to include
        path: String,
    },
    
    /// Native type constructor @type(...)
    Native {
        /// Type name (date, size, duration, etc.)
        type_name: String,
        /// Constructor arguments
        args: Vec<AstNode>,
    },
}

/// Table entry with key information
#[derive(Debug, Clone, PartialEq)]
pub struct TableEntry {
    /// Key with source information
    pub key: Key,
    /// Value node
    pub value: AstNode,
    /// Comments specific to this entry
    pub comments: Comments,
}

/// Key representation with format preservation
#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    /// Key segments (for dotted keys like server.host)
    pub segments: Vec<KeySegment>,
    /// Source span
    pub span: Span,
}

/// A single key segment
#[derive(Debug, Clone, PartialEq)]
pub struct KeySegment {
    /// The key name
    pub name: String,
    /// Whether this segment was quoted
    pub quoted: bool,
    /// Quote style if quoted
    pub quote_style: Option<StringStyle>,
}

/// String quoting styles
#[derive(Debug, Clone, PartialEq)]
pub enum StringStyle {
    /// Double quotes "string"
    Double,
    /// Single quotes 'string'
    Single,
    /// Triple double quotes """string"""
    TripleDouble,
    /// Triple single quotes '''string'''
    TripleSingle,
    /// Raw string r"string" or r#"string"#
    Raw {
        /// Number of `#` symbols used in the raw string delimiter
        hashes: usize,
    },
}

impl Document {
    /// Create a new document
    pub fn new(root: AstNode) -> Self {
        Self {
            root,
            source_path: None,
            source_text: None,
        }
    }

    /// Create a document with source information
    pub fn with_source(root: AstNode, path: Option<String>, text: Option<String>) -> Self {
        Self {
            root,
            source_path: path,
            source_text: text,
        }
    }

    /// Convert the AST to a Value tree (losing source information)
    pub fn to_value(&self) -> Result<Value> {
        self.root.to_value()
    }

    /// Get the source text for a span
    pub fn source_text_for_span(&self, span: &Span) -> Option<&str> {
        self.source_text
            .as_ref()
            .and_then(|text| text.get(span.start..span.end))
    }

    /// Find the AST node at a given byte offset
    pub fn node_at_offset(&self, offset: usize) -> Option<&AstNode> {
        self.root.find_node_at_offset(offset)
    }

    /// Get all comments in the document
    pub fn all_comments(&self) -> Vec<&Comment> {
        let mut comments = Vec::new();
        self.root.collect_comments(&mut comments);
        comments
    }
}

impl AstNode {
    /// Create a new AST node
    pub fn new(value: AstValue, span: Span) -> Self {
        Self {
            value,
            span,
            comments: Comments::default(),
        }
    }

    /// Create an AST node with comments
    pub fn with_comments(value: AstValue, span: Span, comments: Comments) -> Self {
        Self {
            value,
            span,
            comments,
        }
    }

    /// Convert this AST node to a Value (losing source information)
    pub fn to_value(&self) -> Result<Value> {
        match &self.value {
            AstValue::Null => Ok(Value::Null),
            AstValue::Bool(b) => Ok(Value::Bool(*b)),
            AstValue::Integer { value, .. } => Ok(Value::Integer(*value)),
            AstValue::Float { value, .. } => Ok(Value::Float(*value)),
            AstValue::String { value, .. } => Ok(Value::String(value.clone())),
            AstValue::Array { elements, .. } => {
                let values = elements
                    .iter()
                    .map(|elem| elem.to_value())
                    .collect::<Result<Vec<_>>>()?;
                Ok(Value::Array(values))
            }
            AstValue::Table { entries, .. } => {
                let mut value = Value::Table(BTreeMap::new());
                for entry in entries {
                    let key = entry.key.to_string();
                    let entry_value = entry.value.to_value()?;
                    value.set(&key, entry_value)?;
                }
                Ok(value)
            }
            AstValue::FunctionCall { name, args } => {
                // Handle built-in functions
                match name.as_str() {
                    "env" => self.handle_env_function(args),
                    "size" => self.handle_size_function(args),
                    "duration" => self.handle_duration_function(args),
                    _ => Err(NomlError::validation(format!(
                        "Unknown function: {name}"
                    ))),
                }
            }
            AstValue::Interpolation { path } => {
                // This should be resolved during processing
                Err(NomlError::interpolation(
                    "Unresolved interpolation",
                    path.clone(),
                ))
            }
            AstValue::Include { path } => {
                // This should be resolved during processing
                Err(NomlError::import(
                    path.clone(),
                    "Unresolved include directive",
                ))
            }
            AstValue::Native { type_name, args } => {
                self.handle_native_type(type_name, args)
            }
        }
    }

    /// Find the AST node that contains the given byte offset
    pub fn find_node_at_offset(&self, offset: usize) -> Option<&AstNode> {
        // Check if this node contains the offset
        if offset < self.span.start || offset >= self.span.end {
            return None;
        }

        // Search children first (most specific match)
        match &self.value {
            AstValue::Array { elements, .. } => {
                for element in elements {
                    if let Some(node) = element.find_node_at_offset(offset) {
                        return Some(node);
                    }
                }
            }
            AstValue::Table { entries, .. } => {
                for entry in entries {
                    if let Some(node) = entry.value.find_node_at_offset(offset) {
                        return Some(node);
                    }
                }
            }
            AstValue::FunctionCall { args, .. } => {
                for arg in args {
                    if let Some(node) = arg.find_node_at_offset(offset) {
                        return Some(node);
                    }
                }
            }
            AstValue::Native { args, .. } => {
                for arg in args {
                    if let Some(node) = arg.find_node_at_offset(offset) {
                        return Some(node);
                    }
                }
            }
            _ => {}
        }
        // If no child matches, this node is the match
        Some(self)
    }

    /// Collect all comments recursively from this node and its children
    pub fn collect_comments<'a>(&'a self, comments: &mut Vec<&'a Comment>) {
        // Add comments from this node
        comments.extend(self.comments.before.iter());
        if let Some(ref inline) = self.comments.inline {
            comments.push(inline);
        }
        comments.extend(self.comments.after.iter());

        // Recursively collect from children
        match &self.value {
            AstValue::Array { elements, .. } => {
                for element in elements {
                    element.collect_comments(comments);
                }
            }
            AstValue::Table { entries, .. } => {
                for entry in entries {
                    // Entry-specific comments
                    comments.extend(entry.comments.before.iter());
                    if let Some(ref inline) = entry.comments.inline {
                        comments.push(inline);
                    }
                    comments.extend(entry.comments.after.iter());

                    // Recursively collect from value
                    entry.value.collect_comments(comments);
                }
            }
            AstValue::FunctionCall { args, .. } => {
                for arg in args {
                    arg.collect_comments(comments);
                }
            }
            AstValue::Native { args, .. } => {
                for arg in args {
                    arg.collect_comments(comments);
                }
            }
            _ => {}
        }
    }

    /// Handle env() function calls
    fn handle_env_function(&self, args: &[AstNode]) -> Result<Value> {
        if args.is_empty() || args.len() > 2 {
            return Err(NomlError::validation(
                "env() function requires 1 or 2 arguments"
            ));
        }

        let var_name = match args[0].to_value()? {
            Value::String(name) => name,
            _ => return Err(NomlError::validation(
                "env() first argument must be a string"
            )),
        };

        match std::env::var(&var_name) {
            Ok(value) => Ok(Value::String(value)),
            Err(_) => {
                if args.len() == 2 {
                    // Use default value
                    args[1].to_value()
                } else {
                    Err(NomlError::env_var(var_name, false))
                }
            }
        }
    }

    /// Handle size() function calls
    fn handle_size_function(&self, args: &[AstNode]) -> Result<Value> {
        if args.len() != 1 {
            return Err(NomlError::validation(
                "size() function requires exactly 1 argument"
            ));
        }

        let size_str = match args[0].to_value()? {
            Value::String(s) => s,
            _ => return Err(NomlError::validation(
                "size() argument must be a string"
            )),
        };

        parse_size(&size_str)
            .map(Value::Size)
            .ok_or_else(|| NomlError::validation(format!(
                "Invalid size format: {size_str}"
            )))
    }

    /// Handle duration() function calls
    fn handle_duration_function(&self, args: &[AstNode]) -> Result<Value> {
        if args.len() != 1 {
            return Err(NomlError::validation(
                "duration() function requires exactly 1 argument"
            ));
        }

        let duration_str = match args[0].to_value()? {
            Value::String(s) => s,
            _ => {
                return Err(NomlError::validation(
                    "duration() argument must be a string"
                ))
            }
        };

        parse_duration(&duration_str)
            .map(Value::Duration)
            .ok_or_else(|| NomlError::validation(format!(
                "Invalid duration format: {duration_str}"
            )))
    }

   /// Handle native type constructors
    fn handle_native_type(&self, type_name: &str, args: &[AstNode]) -> Result<Value> {
        match type_name {
            "size" => self.handle_size_function(args),
            "duration" => self.handle_duration_function(args),
            // "date" => self.handle_date_function(args), // Disabled: chrono feature not available
            _ => Err(NomlError::validation(format!(
                "Unknown native type: @{type_name}"
            ))),
        }
    }
}

impl Span {
    /// Create a new span
    pub fn new(start: usize, end: usize, start_line: usize, start_column: usize, end_line: usize, end_column: usize) -> Self {
        Self {
            start,
            end,
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    /// Create a span that covers multiple spans
    pub fn merge(&self, other: &Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            start_line: self.start_line.min(other.start_line),
            start_column: if self.start_line < other.start_line {
                self.start_column
            } else if self.start_line > other.start_line {
                other.start_column
            } else {
                self.start_column.min(other.start_column)
            },
            end_line: self.end_line.max(other.end_line),
            end_column: if self.end_line > other.end_line {
                self.end_column
            } else if self.end_line < other.end_line {
                other.end_column
            } else {
                self.end_column.max(other.end_column)
            },
        }
    }

    /// Check if this span contains a byte offset
    pub fn contains(&self, offset: usize) -> bool {
        offset >= self.start && offset < self.end
    }
}

impl Key {
    /// Create a simple key from a string
    pub fn simple(name: String, span: Span) -> Self {
        Self {
            segments: vec![KeySegment {
                name,
                quoted: false,
                quote_style: None,
            }],
            span,
        }
    }

    /// Create a dotted key from segments
    pub fn dotted(segments: Vec<KeySegment>, span: Span) -> Self {
        Self { segments, span }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, segment) in self.segments.iter().enumerate() {
            if i > 0 {
                write!(f, ".")?;
            }
            if segment.quoted {
                write!(f, "\"{}\"", segment.name)?;
            } else {
                write!(f, "{}", segment.name)?;
            }
        }
        Ok(())
    }
}

impl Comments {
    /// Create empty comments
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if there are any comments
    pub fn is_empty(&self) -> bool {
        self.before.is_empty() && self.inline.is_none() && self.after.is_empty()
    }

    /// Add a comment before this node
    pub fn add_before(&mut self, comment: Comment) {
        self.before.push(comment);
    }

    /// Set the inline comment
    pub fn set_inline(&mut self, comment: Comment) {
        self.inline = Some(comment);
    }

    /// Add a comment after this node
    pub fn add_after(&mut self, comment: Comment) {
        self.after.push(comment);
    }
}

/// Parse a size string like "10MB", "1.5GB" into bytes
fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }

    let (number_part, unit_part) = if let Some(pos) = s.find(|c: char| c.is_alphabetic()) {
        s.split_at(pos)
    } else {
        (s.as_str(), "")
    };

    let number: f64 = number_part.parse().ok()?;
    if number < 0.0 {
        return None;
    }

    let multiplier = match unit_part {
        "" | "b" | "byte" | "bytes" => 1,
        "k" | "kb" | "kib" => 1024,
        "m" | "mb" | "mib" => 1024 * 1024,
        "g" | "gb" | "gib" => 1024 * 1024 * 1024,
        "t" | "tb" | "tib" => 1024_u64.pow(4),
        "p" | "pb" | "pib" => 1024_u64.pow(5),
        _ => return None,
    };

    Some((number * multiplier as f64) as u64)
}

/// Parse a duration string like "30s", "1.5m" into seconds
fn parse_duration(s: &str) -> Option<f64> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }

    let (number_part, unit_part) = if let Some(pos) = s.find(|c: char| c.is_alphabetic()) {
        s.split_at(pos)
    } else {
        (s.as_str(), "s") // Default to seconds
    };

    let number: f64 = number_part.parse().ok()?;
    if number < 0.0 {
        return None;
    }

    let multiplier = match unit_part {
        "ms" | "millisecond" | "milliseconds" => 0.001,
        "" | "s" | "sec" | "second" | "seconds" => 1.0,
        "m" | "min" | "minute" | "minutes" => 60.0,
        "h" | "hr" | "hour" | "hours" => 3600.0,
        "d" | "day" | "days" => 86400.0,
        "w" | "week" | "weeks" => 604800.0,
        _ => return None,
    };

    Some(number * multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_operations() {
        let span1 = Span::new(10, 20, 1, 10, 1, 20);
        let span2 = Span::new(15, 25, 1, 15, 1, 25);
        let merged = span1.merge(&span2);
        
        assert_eq!(merged.start, 10);
        assert_eq!(merged.end, 25);
        assert!(merged.contains(12));
        assert!(!merged.contains(5));
    }

    #[test]
    fn key_display() {
        let key = Key::simple("server".to_string(), Span::new(0, 6, 1, 1, 1, 6));
        assert_eq!(key.to_string(), "server");

        let dotted_key = Key::dotted(
            vec![
                KeySegment {
                    name: "server".to_string(),
                    quoted: false,
                    quote_style: None,
                },
                KeySegment {
                    name: "host".to_string(),
                    quoted: false,
                    quote_style: None,
                },
            ],
            Span::new(0, 11, 1, 1, 1, 11),
        );
        assert_eq!(dotted_key.to_string(), "server.host");
    }

    #[test]
    fn parse_sizes() {
        assert_eq!(parse_size("1024"), Some(1024));
        assert_eq!(parse_size("1KB"), Some(1024));
        assert_eq!(parse_size("1.5MB"), Some(1572864));
        assert_eq!(parse_size("1GB"), Some(1073741824));
        assert_eq!(parse_size("invalid"), None);
    }

    #[test]
    fn parse_durations() {
        assert_eq!(parse_duration("30"), Some(30.0));
        assert_eq!(parse_duration("30s"), Some(30.0));
        assert_eq!(parse_duration("1.5m"), Some(90.0));
        assert_eq!(parse_duration("2h"), Some(7200.0));
        assert_eq!(parse_duration("invalid"), None);
    }

    #[test]
    fn ast_to_value_conversion() {
        let span = Span::new(0, 4, 1, 1, 1, 4);
        
        // Test simple values
        let bool_node = AstNode::new(AstValue::Bool(true), span.clone());
        assert_eq!(bool_node.to_value().unwrap(), Value::Bool(true));

        let int_node = AstNode::new(
            AstValue::Integer {
                value: 42,
                raw: "42".to_string(),
            },
            span.clone(),
        );
        assert_eq!(int_node.to_value().unwrap(), Value::Integer(42));

        let str_node = AstNode::new(
            AstValue::String {
                value: "hello".to_string(),
                style: StringStyle::Double,
                has_escapes: false,
            },
            span,
        );
        assert_eq!(str_node.to_value().unwrap(), Value::String("hello".to_string()));
    }
}
