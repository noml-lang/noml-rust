//! # Format-Preserving NOML Serializer
//!
//! This module provides serialization capabilities that preserve the original
//! formatting, comments, whitespace, and style of NOML documents. This enables
//! perfect round-trip editing while maintaining the exact appearance of the
//! original file.

use crate::error::Result;
use crate::parser::ast::{
    AstNode, AstValue, Comment, Document, FormatMetadata, FormatStyle, Indentation, Key,
    LineEnding, StringStyle, TableEntry,
};
use std::fmt::Write;

/// A format-preserving serializer for NOML documents
pub struct Serializer {
    /// Output buffer
    output: String,
    /// Current indentation level
    indent_level: usize,
    /// Default indentation configuration
    indentation: Indentation,
    /// Default line ending style
    line_ending: LineEnding,
}

impl Serializer {
    /// Create a new serializer with default formatting
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
            indentation: Indentation::default(),
            line_ending: LineEnding::default(),
        }
    }

    /// Create a serializer with custom formatting options
    pub fn with_options(indentation: Indentation, line_ending: LineEnding) -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
            indentation,
            line_ending,
        }
    }

    /// Serialize a document to a string, preserving all formatting
    pub fn serialize_document(&mut self, document: &Document) -> Result<String> {
        self.output.clear();

        // Add leading whitespace from root format metadata
        self.output
            .push_str(&document.root.format.leading_whitespace);

        // Serialize the root node (typically a table)
        self.serialize_ast_node(&document.root)?;

        // Add trailing whitespace from root format metadata
        self.output
            .push_str(&document.root.format.trailing_whitespace);

        Ok(self.output.clone())
    }

    /// Serialize a single table entry with formatting preservation
    fn serialize_table_entry(&mut self, entry: &TableEntry) -> Result<()> {
        // Add leading whitespace from format metadata
        self.output.push_str(&entry.value.format.leading_whitespace);

        // Add comments before the entry
        for comment in &entry.comments.before {
            self.serialize_comment(comment);
            self.add_line_ending();
        }

        // Serialize the key
        self.serialize_key(&entry.key);

        // Add equals sign with proper spacing
        if let FormatStyle::KeyValue { equals_spacing, .. } = &entry.value.format.format_style {
            self.output.push_str(&equals_spacing.before);
            self.output.push('=');
            self.output.push_str(&equals_spacing.after);
        } else {
            self.output.push_str(" = ");
        }

        // Serialize the value
        self.serialize_ast_node(&entry.value)?;

        // Add inline comment if present
        if let Some(ref comment) = entry.comments.inline {
            self.output.push(' ');
            self.serialize_comment(comment);
        }

        // Add line ending
        self.add_line_ending();

        // Add comments after the entry
        for comment in &entry.comments.after {
            self.serialize_comment(comment);
            self.add_line_ending();
        }

        // Add trailing whitespace
        self.output
            .push_str(&entry.value.format.trailing_whitespace);

        Ok(())
    }

    /// Serialize a key with proper quoting and formatting
    fn serialize_key(&mut self, key: &Key) {
        for (i, segment) in key.segments.iter().enumerate() {
            if i > 0 {
                self.output.push('.');
            }

            if segment.quoted {
                // Use the original quote style if available
                let quote_char = match segment.quote_style {
                    Some(StringStyle::Double) => '"',
                    Some(StringStyle::Single) => '\'',
                    _ => '"', // Default to double quotes
                };
                self.output.push(quote_char);
                self.output.push_str(&segment.name);
                self.output.push(quote_char);
            } else {
                self.output.push_str(&segment.name);
            }
        }
    }

    /// Serialize an AST node with full formatting preservation
    fn serialize_ast_node(&mut self, node: &AstNode) -> Result<()> {
        match &node.value {
            AstValue::Null => self.output.push_str("null"),
            AstValue::Bool(b) => self.output.push_str(&b.to_string()),
            AstValue::Integer { raw, .. } => self.output.push_str(raw),
            AstValue::Float { raw, .. } => self.output.push_str(raw),
            AstValue::String {
                value,
                style,
                has_escapes,
            } => {
                self.serialize_string(value, style, *has_escapes);
            }
            AstValue::Array { elements, .. } => {
                self.serialize_array(elements, &node.format)?;
            }
            AstValue::Table { entries, inline } => {
                if *inline {
                    self.serialize_inline_table(entries)?;
                } else {
                    self.serialize_table(entries)?;
                }
            }
            AstValue::FunctionCall { name, args } => {
                self.serialize_function_call(name, args)?;
            }
            AstValue::Interpolation { path } => {
                // Use unwrap since we control the format
                write!(self.output, "${{{path}}}").unwrap();
            }
            AstValue::Include { path } => {
                write!(self.output, "include \"{path}\"").unwrap();
            }
            AstValue::Native { type_name, args } => {
                write!(self.output, "@{type_name}(").unwrap();
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.serialize_ast_node(arg)?;
                }
                self.output.push(')');
            }
        }
        Ok(())
    }

    /// Serialize a string value with proper quoting and escaping
    fn serialize_string(&mut self, value: &str, style: &StringStyle, has_escapes: bool) {
        match style {
            StringStyle::Double => {
                self.output.push('"');
                if has_escapes {
                    self.escape_string(value, '"');
                } else {
                    self.output.push_str(value);
                }
                self.output.push('"');
            }
            StringStyle::Single => {
                self.output.push('\'');
                if has_escapes {
                    self.escape_string(value, '\'');
                } else {
                    self.output.push_str(value);
                }
                self.output.push('\'');
            }
            StringStyle::TripleDouble => {
                self.output.push_str("\"\"\"");
                self.output.push_str(value);
                self.output.push_str("\"\"\"");
            }
            StringStyle::TripleSingle => {
                self.output.push_str("'''");
                self.output.push_str(value);
                self.output.push_str("'''");
            }
            StringStyle::Raw { hashes } => {
                self.output.push('r');
                for _ in 0..*hashes {
                    self.output.push('#');
                }
                self.output.push('"');
                self.output.push_str(value);
                self.output.push('"');
                for _ in 0..*hashes {
                    self.output.push('#');
                }
            }
        }
    }

    /// Escape a string value for serialization
    fn escape_string(&mut self, value: &str, quote_char: char) {
        for ch in value.chars() {
            match ch {
                '\n' => self.output.push_str("\\n"),
                '\t' => self.output.push_str("\\t"),
                '\r' => self.output.push_str("\\r"),
                '\\' => self.output.push_str("\\\\"),
                '"' if quote_char == '"' => self.output.push_str("\\\""),
                '\'' if quote_char == '\'' => self.output.push_str("\\'"),
                c => self.output.push(c),
            }
        }
    }

    /// Serialize an array with formatting preservation
    fn serialize_array(&mut self, elements: &[AstNode], format: &FormatMetadata) -> Result<()> {
        self.output.push('[');

        if let FormatStyle::Array {
            multiline,
            trailing_comma,
            bracket_spacing,
        } = &format.format_style
        {
            self.output.push_str(&bracket_spacing.after_open);

            if *multiline {
                // Multi-line array format
                self.add_line_ending();
                self.indent_level += 1;

                for (i, element) in elements.iter().enumerate() {
                    self.add_indentation();
                    self.serialize_ast_node(element)?;

                    if i < elements.len() - 1 || *trailing_comma {
                        self.output.push(',');
                    }

                    self.add_line_ending();
                }

                self.indent_level -= 1;
                self.add_indentation();
            } else {
                // Single-line array format
                for (i, element) in elements.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.serialize_ast_node(element)?;
                }

                if *trailing_comma && !elements.is_empty() {
                    self.output.push(',');
                }
            }

            self.output.push_str(&bracket_spacing.before_close);
        } else {
            // Default array formatting
            for (i, element) in elements.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                self.serialize_ast_node(element)?;
            }
        }

        self.output.push(']');
        Ok(())
    }

    /// Serialize an inline table
    fn serialize_inline_table(&mut self, entries: &[TableEntry]) -> Result<()> {
        self.output.push_str("{ ");

        for (i, entry) in entries.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }

            self.serialize_key(&entry.key);
            self.output.push_str(" = ");
            self.serialize_ast_node(&entry.value)?;
        }

        self.output.push_str(" }");
        Ok(())
    }

    /// Serialize a regular table (not used for inline tables)
    fn serialize_table(&mut self, entries: &[TableEntry]) -> Result<()> {
        for entry in entries {
            self.serialize_table_entry(entry)?;
        }
        Ok(())
    }

    /// Serialize a function call
    fn serialize_function_call(&mut self, name: &str, args: &[AstNode]) -> Result<()> {
        write!(self.output, "{name}(").unwrap();

        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            self.serialize_ast_node(arg)?;
        }

        self.output.push(')');
        Ok(())
    }

    /// Serialize a comment
    fn serialize_comment(&mut self, comment: &Comment) {
        self.output.push('#');
        if !comment.text.is_empty() {
            self.output.push(' ');
            self.output.push_str(&comment.text);
        }
    }

    /// Add appropriate line ending
    fn add_line_ending(&mut self) {
        match self.line_ending {
            LineEnding::Unix => self.output.push('\n'),
            LineEnding::Windows => self.output.push_str("\r\n"),
            LineEnding::Mac => self.output.push('\r'),
        }
    }

    /// Add indentation for the current level
    fn add_indentation(&mut self) {
        let indent_str = if self.indentation.use_tabs {
            "\t".repeat(self.indent_level)
        } else {
            " ".repeat(self.indent_level * self.indentation.size)
        };
        self.output.push_str(&indent_str);
    }
}

impl Default for Serializer {
    fn default() -> Self {
        Self::new()
    }
}

/// High-level function to serialize a document with format preservation
pub fn serialize_document(document: &Document) -> Result<String> {
    let mut serializer = Serializer::new();
    serializer.serialize_document(document)
}

/// High-level function to serialize a document with custom formatting
pub fn serialize_document_with_options(
    document: &Document,
    indentation: Indentation,
    line_ending: LineEnding,
) -> Result<String> {
    let mut serializer = Serializer::with_options(indentation, line_ending);
    serializer.serialize_document(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::*;

    #[test]
    fn test_serialize_simple_values() {
        let mut serializer = Serializer::new();

        // Test null
        let null_node = AstNode::new(AstValue::Null, Span::default());
        serializer.serialize_ast_node(&null_node).unwrap();
        assert_eq!(serializer.output, "null");

        serializer.output.clear();

        // Test boolean
        let bool_node = AstNode::new(AstValue::Bool(true), Span::default());
        serializer.serialize_ast_node(&bool_node).unwrap();
        assert_eq!(serializer.output, "true");
    }

    #[test]
    fn test_serialize_string_with_escapes() {
        let mut serializer = Serializer::new();

        let string_node = AstNode::new(
            AstValue::String {
                value: "hello\nworld".to_string(),
                style: StringStyle::Double,
                has_escapes: true,
            },
            Span::default(),
        );

        serializer.serialize_ast_node(&string_node).unwrap();
        assert_eq!(serializer.output, "\"hello\\nworld\"");
    }

    #[test]
    fn test_serialize_raw_string() {
        let mut serializer = Serializer::new();

        let string_node = AstNode::new(
            AstValue::String {
                value: "no\\escapes".to_string(),
                style: StringStyle::Raw { hashes: 0 },
                has_escapes: false,
            },
            Span::default(),
        );

        serializer.serialize_ast_node(&string_node).unwrap();
        assert_eq!(serializer.output, "r\"no\\escapes\"");
    }
}
