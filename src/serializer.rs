//! # Format-Preserving NOML Serializer
//!
//! This module writes a [`Document`] back to NOML text. It keeps what the
//! parser records: key order, comments (above an entry and at the end of a
//! line), table and array-of-tables sections, quote styles, and the original
//! spelling of numbers (`0xFF`, `1_000`). Whitespace and indentation are
//! normalised. The output always parses back to the same values.

use crate::error::Result;
use crate::parser::ast::{
    AstNode, AstValue, Comment, Document, Indentation, Key, KeySegment, LineEnding, StringStyle,
    TableEntry,
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

/// True for a table written as a `[section]`
fn is_section(node: &AstNode) -> bool {
    matches!(node.value, AstValue::Table { inline: false, .. })
}

/// True for an array written as `[[section]]` blocks
fn is_section_array(node: &AstNode) -> bool {
    matches!(&node.value, AstValue::Array { elements, .. }
        if !elements.is_empty() && elements.iter().all(is_section))
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

    /// Serialize a document to a string
    pub fn serialize_document(&mut self, document: &Document) -> Result<String> {
        self.output.clear();
        self.indent_level = 0;

        let root = &document.root;
        self.output.push_str(&root.format.leading_whitespace);
        for comment in &root.comments.before {
            self.serialize_comment(comment);
            self.add_line_ending();
        }

        match &root.value {
            AstValue::Table { entries, .. } => self.serialize_section_body(entries, &[])?,
            _ => {
                self.serialize_ast_node(root)?;
                self.add_line_ending();
            }
        }

        for comment in &root.comments.after {
            self.serialize_comment(comment);
            self.add_line_ending();
        }
        self.output.push_str(&root.format.trailing_whitespace);

        Ok(std::mem::take(&mut self.output))
    }

    /// Write a table's entries: plain `key = value` lines first, then nested
    /// `[section]` and `[[section]]` blocks. `path` is the section's own key path.
    fn serialize_section_body(
        &mut self,
        entries: &[TableEntry],
        path: &[KeySegment],
    ) -> Result<()> {
        for entry in entries {
            if !is_section(&entry.value) && !is_section_array(&entry.value) {
                self.serialize_table_entry(entry)?;
            }
        }

        for entry in entries {
            let mut full: Vec<KeySegment> = path.to_vec();
            full.extend(entry.key.segments.iter().cloned());

            if let AstValue::Table {
                entries: inner,
                inline: false,
            } = &entry.value.value
            {
                self.serialize_header(&full, entry, false);
                self.serialize_section_body(inner, &full)?;
            } else if is_section_array(&entry.value) {
                if let AstValue::Array { elements, .. } = &entry.value.value {
                    for (i, element) in elements.iter().enumerate() {
                        if i == 0 {
                            self.serialize_header(&full, entry, true);
                        } else {
                            self.start_block();
                            self.output.push_str("[[");
                            self.serialize_key_segments(&full);
                            self.output.push_str("]]");
                            self.add_line_ending();
                        }
                        if let AstValue::Table { entries: inner, .. } = &element.value {
                            self.serialize_section_body(inner, &full)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Blank line between blocks (but not at the very start)
    fn start_block(&mut self) {
        if !self.output.is_empty() && !self.output.ends_with("\n\n") {
            self.add_line_ending();
        }
    }

    fn serialize_header(&mut self, path: &[KeySegment], entry: &TableEntry, array: bool) {
        self.start_block();
        for comment in &entry.comments.before {
            self.serialize_comment(comment);
            self.add_line_ending();
        }
        self.output.push_str(if array { "[[" } else { "[" });
        self.serialize_key_segments(path);
        self.output.push_str(if array { "]]" } else { "]" });
        if let Some(ref comment) = entry.comments.inline {
            self.output.push(' ');
            self.serialize_comment(comment);
        }
        self.add_line_ending();
    }

    /// Serialize a single `key = value` line with its comments
    fn serialize_table_entry(&mut self, entry: &TableEntry) -> Result<()> {
        for comment in &entry.comments.before {
            self.serialize_comment(comment);
            self.add_line_ending();
        }

        self.serialize_key(&entry.key);
        self.output.push_str(" = ");
        self.serialize_ast_node(&entry.value)?;

        if let Some(ref comment) = entry.comments.inline {
            self.output.push(' ');
            self.serialize_comment(comment);
        }
        self.add_line_ending();

        for comment in &entry.comments.after {
            self.serialize_comment(comment);
            self.add_line_ending();
        }
        Ok(())
    }

    /// Serialize a key with proper quoting
    fn serialize_key(&mut self, key: &Key) {
        self.serialize_key_segments(&key.segments);
    }

    fn serialize_key_segments(&mut self, segments: &[KeySegment]) {
        for (i, segment) in segments.iter().enumerate() {
            if i > 0 {
                self.output.push('.');
            }
            let bare_ok = crate::tree::is_bare_key(&segment.name);
            if !segment.quoted && bare_ok {
                self.output.push_str(&segment.name);
            } else if segment.quote_style == Some(StringStyle::Single)
                && literal_ok(&segment.name, false)
            {
                self.output.push('\'');
                self.output.push_str(&segment.name);
                self.output.push('\'');
            } else {
                self.write_basic_string(&segment.name, false);
            }
        }
    }

    /// Serialize an AST node in value position
    fn serialize_ast_node(&mut self, node: &AstNode) -> Result<()> {
        match &node.value {
            AstValue::Null => self.output.push_str("null"),
            AstValue::Bool(b) => self.output.push_str(if *b { "true" } else { "false" }),
            AstValue::Integer { value, raw } => {
                if raw.is_empty() {
                    let _ = write!(self.output, "{value}");
                } else {
                    self.output.push_str(raw);
                }
            }
            AstValue::Float { value, raw } => {
                if !raw.is_empty() {
                    self.output.push_str(raw);
                } else if value.is_nan() {
                    self.output.push_str("nan");
                } else if value.is_infinite() {
                    self.output
                        .push_str(if *value > 0.0 { "inf" } else { "-inf" });
                } else {
                    let _ = write!(self.output, "{value:?}");
                }
            }
            AstValue::DateTime { value, raw } => {
                if raw.is_empty() {
                    let _ = write!(self.output, "{value}");
                } else {
                    self.output.push_str(raw);
                }
            }
            AstValue::String { value, style, .. } => {
                self.serialize_string(value, style);
            }
            AstValue::Array {
                elements,
                multiline,
                trailing_comma,
            } => {
                self.serialize_array(node, elements, *multiline, *trailing_comma)?;
            }
            AstValue::Table { entries, .. } => {
                // In value position every table is written inline
                self.serialize_inline_table(entries)?;
            }
            AstValue::FunctionCall { name, args } => {
                self.output.push_str(name);
                self.serialize_args(args)?;
            }
            AstValue::Interpolation { path } => {
                self.output.push_str("${");
                self.output.push_str(path);
                self.output.push('}');
            }
            AstValue::Include { path } => {
                self.output.push_str("include ");
                self.write_basic_string(path, false);
            }
            AstValue::Native { type_name, args } => {
                self.output.push('@');
                self.output.push_str(type_name);
                self.serialize_args(args)?;
            }
        }
        Ok(())
    }

    fn serialize_args(&mut self, args: &[AstNode]) -> Result<()> {
        self.output.push('(');
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            self.serialize_ast_node(arg)?;
        }
        self.output.push(')');
        Ok(())
    }

    /// Serialize a string in its original style when the value allows it,
    /// otherwise as an escaped double-quoted string.
    ///
    /// Values in the AST are interpolation templates, so `${` is written as is.
    fn serialize_string(&mut self, value: &str, style: &StringStyle) {
        match style {
            StringStyle::Double => self.write_basic_string(value, false),
            StringStyle::TripleDouble => {
                self.output.push_str("\"\"\"");
                if value.starts_with('\n') || value.starts_with("\r\n") {
                    // The parser drops a newline right after the opening quotes
                    self.output.push('\n');
                }
                for ch in value.chars() {
                    match ch {
                        '\\' => self.output.push_str("\\\\"),
                        '"' => self.output.push_str("\\\""),
                        '\n' | '\t' | '\r' => self.output.push(ch),
                        c if c.is_control() => {
                            let _ = write!(self.output, "\\u{{{:x}}}", c as u32);
                        }
                        c => self.output.push(c),
                    }
                }
                self.output.push_str("\"\"\"");
            }
            StringStyle::Single if literal_ok(value, false) => {
                self.output.push('\'');
                self.output.push_str(value);
                self.output.push('\'');
            }
            StringStyle::TripleSingle if literal_ok(value, true) => {
                self.output.push_str("'''");
                if value.starts_with('\n') || value.starts_with("\r\n") {
                    self.output.push('\n');
                }
                self.output.push_str(value);
                self.output.push_str("'''");
            }
            StringStyle::Raw { hashes } if raw_ok(value, *hashes) => {
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
            // The value cannot be written in its original style; fall back to
            // an escaped basic string. Literal and raw strings are never
            // interpolated, so protect `${` from being read as interpolation.
            _ => self.write_basic_string(value, true),
        }
    }

    /// Write `"..."` with escapes. With `protect_templates`, `${` is written as
    /// `$${` so it reads back as literal text.
    fn write_basic_string(&mut self, value: &str, protect_templates: bool) {
        self.output.push('"');
        let mut chars = value.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '\n' => self.output.push_str("\\n"),
                '\t' => self.output.push_str("\\t"),
                '\r' => self.output.push_str("\\r"),
                '\\' => self.output.push_str("\\\\"),
                '"' => self.output.push_str("\\\""),
                '$' if protect_templates && chars.peek() == Some(&'{') => {
                    self.output.push_str("$$")
                }
                c if c.is_control() => {
                    let _ = write!(self.output, "\\u{{{:x}}}", c as u32);
                }
                c => self.output.push(c),
            }
        }
        self.output.push('"');
    }

    /// Serialize an array, one element per line if it was written that way
    fn serialize_array(
        &mut self,
        node: &AstNode,
        elements: &[AstNode],
        multiline: bool,
        trailing_comma: bool,
    ) -> Result<()> {
        let has_comments = !node.comments.after.is_empty()
            || elements
                .iter()
                .any(|e| !e.comments.before.is_empty() || e.comments.inline.is_some());
        self.output.push('[');

        if multiline || has_comments {
            self.indent_level += 1;
            self.add_line_ending();
            for (i, element) in elements.iter().enumerate() {
                for comment in &element.comments.before {
                    self.add_indentation();
                    self.serialize_comment(comment);
                    self.add_line_ending();
                }
                self.add_indentation();
                self.serialize_ast_node(element)?;
                // Always end lines with a comma so inline comments stay attached
                if i + 1 < elements.len() || trailing_comma || element.comments.inline.is_some() {
                    self.output.push(',');
                }
                if let Some(ref comment) = element.comments.inline {
                    self.output.push(' ');
                    self.serialize_comment(comment);
                }
                self.add_line_ending();
            }
            for comment in &node.comments.after {
                self.add_indentation();
                self.serialize_comment(comment);
                self.add_line_ending();
            }
            self.indent_level -= 1;
            self.add_indentation();
        } else {
            for (i, element) in elements.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                self.serialize_ast_node(element)?;
            }
            if trailing_comma && !elements.is_empty() {
                self.output.push(',');
            }
        }

        self.output.push(']');
        Ok(())
    }

    /// Serialize an inline table
    fn serialize_inline_table(&mut self, entries: &[TableEntry]) -> Result<()> {
        if entries.is_empty() {
            self.output.push_str("{}");
            return Ok(());
        }
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
        if self.indentation.use_tabs {
            for _ in 0..self.indent_level {
                self.output.push('\t');
            }
        } else {
            for _ in 0..self.indent_level * self.indentation.size {
                self.output.push(' ');
            }
        }
    }
}

/// Can `value` be written as a literal (single-quoted) string?
fn literal_ok(value: &str, multiline: bool) -> bool {
    if multiline {
        !value.contains("'''")
            && !value.ends_with('\'')
            && !value
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\t' && c != '\r')
    } else {
        !value.contains('\'') && !value.chars().any(|c| c.is_control() && c != '\t')
    }
}

/// Can `value` be written as a raw string with `hashes` hashes?
fn raw_ok(value: &str, hashes: usize) -> bool {
    let closing: String = std::iter::once('"')
        .chain(std::iter::repeat_n('#', hashes))
        .collect();
    !value.contains(&closing)
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
