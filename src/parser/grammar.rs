//! # NOML Grammar and Parser
//!
//! This module implements the NOML parser using a hand-written recursive descent parser.
//! While we planned to use chumsky, for the MVP we'll implement a direct parser for speed
//! and to avoid complex combinator setup. Future versions can migrate to chumsky for
//! more advanced error recovery.

use crate::error::{NomlError, Result};
use crate::parser::ast::{
    AstNode, AstValue, Comment, CommentStyle, Comments, Document, Key, KeySegment, Span,
    StringStyle, TableEntry,
};
use crate::parser::lexer::{Lexer, StringStyle as LexerStringStyle, Token, TokenKind};
use std::fs;
use std::path::Path;

/// Parse NOML from a string with optional source path
pub fn parse_string(source: &str, source_path: Option<String>) -> Result<Document> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;

    let mut parser = NomlParser::new(tokens, source);
    let mut document = parser.parse()?;

    // Set source information
    document.source_path = source_path;
    document.source_text = Some(source.to_string());

    Ok(document)
}

/// Parse NOML from a file
pub fn parse_file(path: &Path) -> Result<Document> {
    let source = fs::read_to_string(path)
        .map_err(|e| NomlError::io(path.to_string_lossy().to_string(), e))?;

    parse_string(&source, Some(path.to_string_lossy().to_string()))
}

/// Parse NOML from a file asynchronously
#[cfg(feature = "async")]
pub async fn parse_file_async(path: &std::path::Path) -> Result<Document> {
    let source = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| NomlError::io(path.to_string_lossy().to_string(), e))?;

    parse_string(&source, Some(path.to_string_lossy().to_string()))
}

/// NOML parser implementation
pub struct NomlParser<'a> {
    /// Input tokens
    tokens: Vec<Token<'a>>,
    /// Current position in token stream
    pos: usize,
    /// Source text for span calculations
    source: &'a str,
}

impl<'a> NomlParser<'a> {
    /// Create a new parser
    pub fn new(tokens: Vec<Token<'a>>, source: &'a str) -> Self {
        Self {
            tokens,
            pos: 0,
            source,
        }
    }

    /// Parse the tokens into a Document
    pub fn parse(&mut self) -> Result<Document> {
        let root_node = self.parse_document()?;
        Ok(Document::new(root_node))
    }

    /// Parse a complete document (top-level table)
    fn parse_document(&mut self) -> Result<AstNode> {
        let start_span = self.current_span();
        let mut entries = Vec::new();
        let mut comments = Comments::new();

        // Collect leading comments
        self.collect_leading_comments(&mut comments);

        while !self.is_at_end() {
            // Collect any leading comments first
            self.collect_leading_comments(&mut comments);

            // Skip whitespace and newlines
            if self.skip_insignificant_tokens() {
                continue;
            }

            if self.is_at_end() {
                break;
            }

            // Parse table entry or table header
            if self.check_token(&TokenKind::LeftBracket) {
                // Parse table header - this creates nested table structure
                self.parse_table_header(&mut entries)?;
            } else {
                // Parse key-value pair
                let kv_entry = self.parse_key_value_pair()?;
                entries.push(kv_entry);
            }
        }

        let end_span = self.current_span();
        let span = start_span.merge(&end_span);

        let ast_value = AstValue::Table {
            entries,
            inline: false,
        };

        Ok(AstNode::with_comments(ast_value, span, comments))
    }

    /// Parse a table header like `[section]` or `[section.subsection]`
    fn parse_table_header(&mut self, entries: &mut Vec<TableEntry>) -> Result<()> {
        let start_span = self.current_span();
        let mut comments = Comments::new();

        // Collect comments before the header
        self.collect_leading_comments(&mut comments);

        // Consume first '['
        self.consume_token(&TokenKind::LeftBracket, "Expected '['")?;

        // Check for array of tables syntax [[...]]
        let is_array_of_tables = self.check_token(&TokenKind::LeftBracket);
        if is_array_of_tables {
            self.consume_token(&TokenKind::LeftBracket, "Expected second '['")?;
        }

        // Parse the key path
        let key = self.parse_key()?;

        // Consume closing brackets
        if is_array_of_tables {
            self.consume_token(&TokenKind::RightBracket, "Expected first ']'")?;
        }
        self.consume_token(&TokenKind::RightBracket, "Expected ']'")?;

        // Collect inline comment if present
        if let Some(comment) = self.parse_inline_comment()? {
            comments.set_inline(comment);
        }

        // Skip newlines after header
        self.skip_insignificant_tokens();

        // Parse the contents of this table section
        let mut table_entries = Vec::new();
        while !self.is_at_end() && !self.check_token(&TokenKind::LeftBracket) {
            // Collect any comments first
            self.collect_leading_comments(&mut comments);

            if self.skip_insignificant_tokens() {
                continue;
            }

            if self.is_at_end() || self.check_token(&TokenKind::LeftBracket) {
                break;
            }

            let entry = self.parse_key_value_pair()?;
            table_entries.push(entry);
        }

        // Create the table value
        let end_span = self.current_span();
        let table_span = start_span.merge(&end_span);

        let table_value = AstNode::new(
            AstValue::Table {
                entries: table_entries,
                inline: false,
            },
            table_span.clone(),
        );

        // Create the table entry
        // For array of tables, we need special handling to create arrays
        if is_array_of_tables {
            // Check if we already have an entry with this key
            if let Some(existing_entry) = entries
                .iter_mut()
                .find(|e| e.key.to_string() == key.to_string())
            {
                // Convert existing table to array of tables or add to existing array
                match &mut existing_entry.value.value {
                    AstValue::Array { elements, .. } => {
                        // Already an array, add new table
                        elements.push(table_value);
                    }
                    _ => {
                        // Convert single table to array of tables
                        let existing_table = existing_entry.value.clone();
                        let array_value = AstValue::Array {
                            elements: vec![existing_table, table_value],
                            multiline: true,
                            trailing_comma: false,
                        };
                        existing_entry.value =
                            AstNode::new(array_value, existing_entry.value.span.clone());
                    }
                }
            } else {
                // First occurrence - create array with single element
                let array_value = AstValue::Array {
                    elements: vec![table_value],
                    multiline: true,
                    trailing_comma: false,
                };
                let array_node = AstNode::new(array_value, table_span);
                let entry = TableEntry {
                    key,
                    value: array_node,
                    comments,
                };
                entries.push(entry);
            }
        } else {
            // Regular table
            let entry = TableEntry {
                key,
                value: table_value,
                comments,
            };
            entries.push(entry);
        }
        Ok(())
    }

    /// Parse a key-value pair
    fn parse_key_value_pair(&mut self) -> Result<TableEntry> {
        let mut comments = Comments::new();

        // Collect leading comments
        self.collect_leading_comments(&mut comments);

        // Parse the key
        let key = self.parse_key()?;

        // Consume '='
        self.consume_token(&TokenKind::Equals, "Expected '='")?;

        // Parse the value
        let value = self.parse_value()?;

        // Collect inline comment
        if let Some(comment) = self.parse_inline_comment()? {
            comments.set_inline(comment);
        }

        Ok(TableEntry {
            key,
            value,
            comments,
        })
    }

    /// Parse a key (possibly dotted)
    fn parse_key(&mut self) -> Result<Key> {
        let start_span = self.current_span();
        let mut segments = Vec::new();

        // Parse first segment
        let first_segment = self.parse_key_segment()?;
        segments.push(first_segment);

        // Parse additional segments separated by dots
        while self.match_token(&TokenKind::Dot) {
            let segment = self.parse_key_segment()?;
            segments.push(segment);
        }

        let end_span = self.current_span();
        let span = start_span.merge(&end_span);

        Ok(Key::dotted(segments, span))
    }

    /// Parse a single key segment
    fn parse_key_segment(&mut self) -> Result<KeySegment> {
        let token = self.advance()?;

        match &token.kind {
            TokenKind::Identifier(name) => Ok(KeySegment {
                name: name.to_string(),
                quoted: false,
                quote_style: None,
            }),
            TokenKind::String { value, style } => Ok(KeySegment {
                name: value.clone(),
                quoted: true,
                quote_style: Some(convert_string_style(style)),
            }),
            _ => Err(NomlError::unexpected_token(
                format!("{}", token.kind),
                "identifier or string",
                token.span.start_line,
                token.span.start_column,
            )),
        }
    }

    /// Parse a value
    fn parse_value(&mut self) -> Result<AstNode> {
        let token = self.peek()?;

        match &token.kind {
            // Literals
            TokenKind::String { .. } => self.parse_string_value(),
            TokenKind::Integer { .. } => self.parse_integer_value(),
            TokenKind::Float { .. } => self.parse_float_value(),
            TokenKind::Bool(_) => self.parse_bool_value(),
            TokenKind::Null => self.parse_null_value(),

            // Collections
            TokenKind::LeftBracket => self.parse_array(),
            TokenKind::LeftBrace => self.parse_inline_table(),

            // Functions and special constructs
            TokenKind::EnvFunc => self.parse_env_function(),
            TokenKind::At => self.parse_native_type(),
            TokenKind::InterpolationStart => self.parse_interpolation(),
            TokenKind::Include => self.parse_include(),

            _ => Err(NomlError::parse_with_suggestion(
                format!("Unexpected token: {}", token.kind),
                token.span.start_line,
                token.span.start_column,
                "Expected a value (string, number, boolean, array, or table)",
            )),
        }
    }

    /// Parse a string value
    fn parse_string_value(&mut self) -> Result<AstNode> {
        let token = self.advance()?;

        if let TokenKind::String {
            ref value,
            ref style,
        } = token.kind
        {
            // Check if the original raw text contains escape sequences
            let has_escapes = token.text.contains('\\');

            let ast_value = AstValue::String {
                value: value.clone(),
                style: convert_string_style(style),
                has_escapes,
            };
            Ok(AstNode::new(ast_value, token.span.clone()))
        } else {
            unreachable!("Expected string token")
        }
    }

    /// Parse an integer value
    fn parse_integer_value(&mut self) -> Result<AstNode> {
        let token = self.advance()?;

        if let TokenKind::Integer { value, ref raw } = token.kind {
            let ast_value = AstValue::Integer {
                value,
                raw: raw.to_string(),
            };
            Ok(AstNode::new(ast_value, token.span.clone()))
        } else {
            unreachable!("Expected integer token")
        }
    }

    /// Parse a float value
    fn parse_float_value(&mut self) -> Result<AstNode> {
        let token = self.advance()?;

        if let TokenKind::Float { value, ref raw } = token.kind {
            let ast_value = AstValue::Float {
                value,
                raw: raw.to_string(),
            };
            Ok(AstNode::new(ast_value, token.span.clone()))
        } else {
            unreachable!("Expected float token")
        }
    }

    /// Parse a boolean value
    fn parse_bool_value(&mut self) -> Result<AstNode> {
        let token = self.advance()?;

        if let TokenKind::Bool(value) = token.kind {
            let ast_value = AstValue::Bool(value);
            Ok(AstNode::new(ast_value, token.span.clone()))
        } else {
            unreachable!("Expected bool token")
        }
    }

    /// Parse a null value
    fn parse_null_value(&mut self) -> Result<AstNode> {
        let token = self.advance()?;
        let ast_value = AstValue::Null;
        Ok(AstNode::new(ast_value, token.span.clone()))
    }

    /// Parse an array
    fn parse_array(&mut self) -> Result<AstNode> {
        let start_span = self.current_span();

        // Consume '['
        self.consume_token(&TokenKind::LeftBracket, "Expected '['")?;

        let mut elements = Vec::new();
        let mut multiline = false;
        let mut trailing_comma = false;

        // Handle empty array
        if self.match_token(&TokenKind::RightBracket) {
            let end_span = self.current_span();
            let span = start_span.merge(&end_span);

            let ast_value = AstValue::Array {
                elements,
                multiline: false,
                trailing_comma: false,
            };
            return Ok(AstNode::new(ast_value, span));
        }

        // Parse array elements
        loop {
            // Check for newlines (indicates multiline)
            if self.skip_newlines() {
                multiline = true;
            }

            // Check for closing bracket
            if self.check_token(&TokenKind::RightBracket) {
                break;
            }

            // Parse element
            let element = self.parse_value()?;
            elements.push(element);

            // Skip whitespace
            self.skip_whitespace();

            // Check for comma or end
            if self.match_token(&TokenKind::Comma) {
                trailing_comma = true;
                self.skip_whitespace();

                // Check if this was a trailing comma
                if self.check_token(&TokenKind::RightBracket) {
                    break;
                } else {
                    trailing_comma = false;
                }
            } else if self.check_token(&TokenKind::RightBracket) {
                break;
            } else {
                return Err(NomlError::parse(
                    "Expected ',' or ']' in array",
                    self.current_line(),
                    self.current_column(),
                ));
            }
        }

        // Consume ']'
        self.consume_token(&TokenKind::RightBracket, "Expected ']'")?;

        let end_span = self.current_span();
        let span = start_span.merge(&end_span);

        let ast_value = AstValue::Array {
            elements,
            multiline,
            trailing_comma,
        };

        Ok(AstNode::new(ast_value, span))
    }

    /// Parse an inline table
    fn parse_inline_table(&mut self) -> Result<AstNode> {
        let start_span = self.current_span();

        // Consume '{'
        self.consume_token(&TokenKind::LeftBrace, "Expected '{'")?;

        let mut entries = Vec::new();

        // Skip whitespace
        self.skip_whitespace();

        // Handle empty table
        if self.match_token(&TokenKind::RightBrace) {
            let end_span = self.current_span();
            let span = start_span.merge(&end_span);

            let ast_value = AstValue::Table {
                entries,
                inline: true,
            };
            return Ok(AstNode::new(ast_value, span));
        }

        // Parse table entries
        loop {
            // Parse key-value pair
            let entry = self.parse_key_value_pair()?;
            entries.push(entry);

            // Skip whitespace
            self.skip_whitespace();

            // Check for comma or end
            if self.match_token(&TokenKind::Comma) {
                self.skip_whitespace();

                // Check for trailing comma
                if self.check_token(&TokenKind::RightBrace) {
                    break;
                }
            } else if self.check_token(&TokenKind::RightBrace) {
                break;
            } else {
                return Err(NomlError::parse(
                    "Expected ',' or '}' in inline table",
                    self.current_line(),
                    self.current_column(),
                ));
            }
        }

        // Consume '}'
        self.consume_token(&TokenKind::RightBrace, "Expected '}'")?;

        let end_span = self.current_span();
        let span = start_span.merge(&end_span);

        let ast_value = AstValue::Table {
            entries,
            inline: true,
        };

        Ok(AstNode::new(ast_value, span))
    }

    /// Parse env() function
    fn parse_env_function(&mut self) -> Result<AstNode> {
        let start_span = self.current_span();

        // Consume 'env'
        self.consume_token(&TokenKind::EnvFunc, "Expected 'env'")?;

        // Consume '('
        self.consume_token(&TokenKind::LeftParen, "Expected '('")?;

        let mut args = Vec::new();

        // Parse arguments
        if !self.check_token(&TokenKind::RightParen) {
            loop {
                let arg = self.parse_value()?;
                args.push(arg);

                if self.match_token(&TokenKind::Comma) {
                    self.skip_whitespace();
                    if self.check_token(&TokenKind::RightParen) {
                        break; // Trailing comma
                    }
                } else {
                    break;
                }
            }
        }

        // Consume ')'
        self.consume_token(&TokenKind::RightParen, "Expected ')'")?;

        let end_span = self.current_span();
        let span = start_span.merge(&end_span);

        let ast_value = AstValue::FunctionCall {
            name: "env".to_string(),
            args,
        };

        Ok(AstNode::new(ast_value, span))
    }

    /// Parse native type like @size(\"10MB\")
    fn parse_native_type(&mut self) -> Result<AstNode> {
        let start_span = self.current_span();

        // Consume '@'
        self.consume_token(&TokenKind::At, "Expected '@'")?;

        // Parse type name
        let type_name = if let TokenKind::Identifier(name) = &self.advance()?.kind {
            name.to_string()
        } else {
            return Err(NomlError::parse(
                "Expected type name after '@'",
                self.current_line(),
                self.current_column(),
            ));
        };

        // Consume '('
        self.consume_token(&TokenKind::LeftParen, "Expected '('")?;

        let mut args = Vec::new();

        // Parse arguments
        if !self.check_token(&TokenKind::RightParen) {
            loop {
                let arg = self.parse_value()?;
                args.push(arg);

                if self.match_token(&TokenKind::Comma) {
                    self.skip_whitespace();
                    if self.check_token(&TokenKind::RightParen) {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        // Consume ')'
        self.consume_token(&TokenKind::RightParen, "Expected ')'")?;

        let end_span = self.current_span();
        let span = start_span.merge(&end_span);

        let ast_value = AstValue::Native { type_name, args };

        Ok(AstNode::new(ast_value, span))
    }

    /// Parse interpolation ${path}
    fn parse_interpolation(&mut self) -> Result<AstNode> {
        let start_span = self.current_span();

        // Consume '${'
        self.consume_token(&TokenKind::InterpolationStart, "Expected '${'")?;

        // Parse the path - support dot-separated paths and array indices
        let mut path_segments = Vec::new();

        // First segment must be an identifier
        if let TokenKind::Identifier(name) = &self.advance()?.kind {
            path_segments.push(name.to_string());
        } else {
            return Err(NomlError::parse_with_suggestion(
                "Expected identifier in interpolation path",
                self.current_line(),
                self.current_column(),
                "Interpolation paths should start with a variable name (e.g., '${server.host}')",
            ));
        }

        // Parse additional segments separated by dots
        while self.pos < self.tokens.len() {
            if let Ok(token) = self.peek() {
                if token.kind == TokenKind::Dot {
                    self.advance()?; // consume dot

                    // Next token should be identifier or integer (for array access)
                    let next_token = self.advance()?;
                    match &next_token.kind {
                        TokenKind::Identifier(name) => {
                            path_segments.push(name.to_string());
                        }
                        TokenKind::Integer { value, .. } => {
                            path_segments.push(value.to_string());
                        }
                        _ => {
                            return Err(NomlError::parse(
                                "Expected identifier or integer after '.' in path",
                                self.current_line(),
                                self.current_column(),
                            ));
                        }
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Join path segments with dots
        let path = path_segments.join(".");

        // Consume '}'
        self.consume_token(&TokenKind::RightBrace, "Expected '}'")?;

        let end_span = self.current_span();
        let span = start_span.merge(&end_span);

        let ast_value = AstValue::Interpolation { path };

        Ok(AstNode::new(ast_value, span))
    }

    /// Parse include statement
    fn parse_include(&mut self) -> Result<AstNode> {
        let start_span = self.current_span();

        // Consume 'include'
        self.consume_token(&TokenKind::Include, "Expected 'include'")?;

        // Parse the path string
        let path_node = self.parse_string_value()?;
        let path = if let AstValue::String { ref value, .. } = path_node.value {
            value.clone()
        } else {
            return Err(NomlError::parse(
                "Expected string path for include",
                self.current_line(),
                self.current_column(),
            ));
        };

        let end_span = self.current_span();
        let span = start_span.merge(&end_span);

        let ast_value = AstValue::Include { path };

        Ok(AstNode::new(ast_value, span))
    }

    // Helper methods for token management

    /// Check if at end of tokens
    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
            || matches!(self.tokens.get(self.pos), Some(token) if matches!(token.kind, TokenKind::Eof))
    }

    /// Peek at current token
    fn peek(&self) -> Result<&Token> {
        self.tokens
            .get(self.pos)
            .ok_or_else(|| NomlError::parse("Unexpected end of input", 1, 1))
    }

    /// Advance to next token
    fn advance(&mut self) -> Result<Token<'a>> {
        if self.is_at_end() {
            return Err(NomlError::parse(
                "Unexpected end of input",
                self.current_line(),
                self.current_column(),
            ));
        }
        let token = self.tokens[self.pos].clone();
        self.pos += 1;
        Ok(token)
    }

    /// Check if current token matches given kind
    fn check_token(&self, kind: &TokenKind) -> bool {
        if let Ok(token) = self.peek() {
            std::mem::discriminant(&token.kind) == std::mem::discriminant(kind)
        } else {
            false
        }
    }

    /// Match and consume token if it matches
    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check_token(kind) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Consume token or return error
    fn consume_token(&mut self, kind: &TokenKind, message: &str) -> Result<&Token> {
        if self.check_token(kind) {
            let pos = self.pos;
            self.pos += 1;
            Ok(&self.tokens[pos])
        } else {
            let token = self.peek()?;
            Err(NomlError::parse(
                message.to_string(),
                token.span.start_line,
                token.span.start_column,
            ))
        }
    }

    /// Skip whitespace tokens
    fn skip_whitespace(&mut self) -> bool {
        let mut skipped = false;
        while let Ok(token) = self.peek() {
            if matches!(token.kind, TokenKind::Whitespace) {
                self.pos += 1;
                skipped = true;
            } else {
                break;
            }
        }
        skipped
    }

    /// Skip newline tokens
    fn skip_newlines(&mut self) -> bool {
        let mut skipped = false;
        while let Ok(token) = self.peek() {
            if matches!(token.kind, TokenKind::Newline) {
                self.pos += 1;
                skipped = true;
            } else {
                break;
            }
        }
        skipped
    }

    /// Skip whitespace and newline tokens
    fn skip_insignificant_tokens(&mut self) -> bool {
        let mut skipped = false;
        while let Ok(token) = self.peek() {
            match token.kind {
                TokenKind::Whitespace | TokenKind::Newline => {
                    self.pos += 1;
                    skipped = true;
                }
                _ => break,
            }
        }
        skipped
    }

    /// Get current position for span calculation
    fn current_span(&self) -> Span {
        if let Ok(token) = self.peek() {
            token.span.clone()
        } else {
            // End of file span
            Span::new(
                self.source.len(),
                self.source.len(),
                self.current_line(),
                self.current_column(),
                self.current_line(),
                self.current_column(),
            )
        }
    }

    /// Get current line number
    fn current_line(&self) -> usize {
        if let Ok(token) = self.peek() {
            token.span.start_line
        } else {
            // Calculate line from source
            self.source.matches('\n').count() + 1
        }
    }

    /// Get current column number
    fn current_column(&self) -> usize {
        if let Ok(token) = self.peek() {
            token.span.start_column
        } else {
            1
        }
    }

    /// Collect leading comments
    fn collect_leading_comments(&mut self, comments: &mut Comments) {
        while let Ok(token) = self.peek() {
            match &token.kind {
                TokenKind::Comment { text } => {
                    let comment = Comment {
                        text: text.clone(),
                        span: token.span.clone(),
                        style: CommentStyle::Line,
                    };
                    comments.add_before(comment);
                    self.pos += 1;
                }
                TokenKind::Whitespace | TokenKind::Newline => {
                    self.pos += 1;
                }
                _ => break,
            }
        }
    }

    /// Parse inline comment
    fn parse_inline_comment(&mut self) -> Result<Option<Comment>> {
        // Skip whitespace first
        self.skip_whitespace();

        if let Ok(token) = self.peek() {
            if let TokenKind::Comment { text } = &token.kind {
                let comment = Comment {
                    text: text.clone(),
                    span: token.span.clone(),
                    style: CommentStyle::Line,
                };
                self.pos += 1;
                return Ok(Some(comment));
            }
        }

        Ok(None)
    }
}

/// Convert lexer string style to AST string style
fn convert_string_style(style: &LexerStringStyle) -> StringStyle {
    match style {
        LexerStringStyle::Double => StringStyle::Double,
        LexerStringStyle::Single => StringStyle::Single,
        LexerStringStyle::TripleDouble => StringStyle::TripleDouble,
        LexerStringStyle::TripleSingle => StringStyle::TripleSingle,
        LexerStringStyle::Raw { hashes } => StringStyle::Raw { hashes: *hashes },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_values() {
        let source = r#"
name = "test"
version = 1.0
debug = true
data = null
"#;
        let doc = parse_string(source, None).unwrap();
        let value = doc.to_value().unwrap();

        assert_eq!(value.get("name").unwrap().as_string().unwrap(), "test");
        assert_eq!(value.get("version").unwrap().as_float().unwrap(), 1.0);
        assert!(value.get("debug").unwrap().as_bool().unwrap());
        assert!(value.get("data").unwrap().is_null());
    }

    #[test]
    fn parse_arrays() {
        let source = r#"
numbers = [1, 2, 3]
strings = ["a", "b", "c"]
mixed = [1, "two", true, null]
"#;
        let doc = parse_string(source, None).unwrap();
        let value = doc.to_value().unwrap();

        let numbers = value.get("numbers").unwrap().as_array().unwrap();
        assert_eq!(numbers.len(), 3);
        assert_eq!(numbers[0].as_integer().unwrap(), 1);

        let strings = value.get("strings").unwrap().as_array().unwrap();
        assert_eq!(strings.len(), 3);
        assert_eq!(strings[0].as_string().unwrap(), "a");

        let mixed = value.get("mixed").unwrap().as_array().unwrap();
        assert_eq!(mixed.len(), 4);
        assert_eq!(mixed[0].as_integer().unwrap(), 1);
        assert_eq!(mixed[1].as_string().unwrap(), "two");
        assert!(mixed[2].as_bool().unwrap());
        assert!(mixed[3].is_null());
    }

    #[test]
    fn parse_tables() {
        let source = r#"
[database]
host = "localhost"
port = 5432

[server]
host = "0.0.0.0"
port = 8080

[database.pool]
min = 5
max = 20
"#;
        let doc = parse_string(source, None).unwrap();
        let value = doc.to_value().unwrap();

        assert_eq!(
            value.get("database.host").unwrap().as_string().unwrap(),
            "localhost"
        );
        assert_eq!(
            value.get("database.port").unwrap().as_integer().unwrap(),
            5432
        );
        assert_eq!(
            value.get("server.host").unwrap().as_string().unwrap(),
            "0.0.0.0"
        );
        assert_eq!(
            value.get("server.port").unwrap().as_integer().unwrap(),
            8080
        );
        assert_eq!(
            value
                .get("database.pool.min")
                .unwrap()
                .as_integer()
                .unwrap(),
            5
        );
        assert_eq!(
            value
                .get("database.pool.max")
                .unwrap()
                .as_integer()
                .unwrap(),
            20
        );
    }

    #[test]
    fn parse_inline_tables() {
        let source = r#"
point = { x = 1, y = 2 }
color = { r = 255, g = 128, b = 0 }
"#;
        let doc = parse_string(source, None).unwrap();
        let value = doc.to_value().unwrap();

        assert_eq!(value.get("point.x").unwrap().as_integer().unwrap(), 1);
        assert_eq!(value.get("point.y").unwrap().as_integer().unwrap(), 2);
        assert_eq!(value.get("color.r").unwrap().as_integer().unwrap(), 255);
        assert_eq!(value.get("color.g").unwrap().as_integer().unwrap(), 128);
        assert_eq!(value.get("color.b").unwrap().as_integer().unwrap(), 0);
    }

    #[test]
    fn parse_comments() {
        let source = r#"
# This is a comment
name = "test" # Inline comment

# Another comment
[section]
# Comment in section
key = "value"
"#;

        let doc = parse_string(source, None).unwrap();
        let comments = doc.all_comments();

        assert!(!comments.is_empty());
        assert!(comments
            .iter()
            .any(|c| c.text.contains("This is a comment")));
        assert!(comments.iter().any(|c| c.text.contains("Inline comment")));
        assert!(comments.iter().any(|c| c.text.contains("Another comment")));
        assert!(comments
            .iter()
            .any(|c| c.text.contains("Comment in section")));
    }
}
