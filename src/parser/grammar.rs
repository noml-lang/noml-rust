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
    /// Current nesting depth of arrays, inline tables and function arguments
    depth: usize,
}

/// Maximum nesting depth for arrays, inline tables and call arguments.
///
/// Deeply nested input would otherwise overflow the stack and abort the
/// process. 64 levels is far beyond what a real config needs and fits in a
/// 1 MiB stack even in debug builds. The resolver applies the same limit to
/// the combined depth of a document and the files it includes.
pub(crate) const MAX_NESTING_DEPTH: usize = 64;

impl<'a> NomlParser<'a> {
    /// Create a new parser
    pub fn new(tokens: Vec<Token<'a>>, source: &'a str) -> Self {
        Self {
            tokens,
            pos: 0,
            source,
            depth: 0,
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

        // Comments are attached to the entry or header that follows them
        let mut leading = self.take_comments();
        while !self.is_at_end() {
            if self.check_token(&TokenKind::LeftBracket) {
                leading = self.parse_table_header(&mut entries, leading)?;
            } else {
                let entry = self.parse_key_value_pair(leading)?;
                entries.push(entry);
                leading = self.take_comments();
            }
        }
        // Comments after the last entry
        comments.after = leading;

        let end_span = self.current_span();
        let span = start_span.merge(&end_span);

        let ast_value = AstValue::Table {
            entries,
            inline: false,
        };

        Ok(AstNode::with_comments(ast_value, span, comments))
    }

    /// Parse a table header like `[section]` or `[section.subsection]` and the
    /// entries below it. Returns the comments that follow the section, which
    /// belong to whatever comes next.
    fn parse_table_header(
        &mut self,
        entries: &mut Vec<TableEntry>,
        leading: Vec<Comment>,
    ) -> Result<Vec<Comment>> {
        let start_span = self.current_span();
        let mut comments = Comments::new();
        comments.before = leading;

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
            self.consume_token(&TokenKind::RightBracket, "Expected ']]'")?;
        }
        self.consume_token(&TokenKind::RightBracket, "Expected ']'")?;

        // Collect inline comment if present
        if let Some(comment) = self.parse_inline_comment()? {
            comments.set_inline(comment);
        }

        // Parse the contents of this table section
        let mut table_entries = Vec::new();
        let mut leading = self.take_comments();
        while !self.is_at_end() && !self.check_token(&TokenKind::LeftBracket) {
            let entry = self.parse_key_value_pair(leading)?;
            table_entries.push(entry);
            leading = self.take_comments();
        }

        // Create the table value
        let end_span = self.current_span();
        let table_span = start_span.merge(&end_span);

        let table_value = AstNode::new(
            AstValue::Table {
                entries: table_entries,
                inline: false,
            },
            table_span,
        );

        attach_section(entries, key, table_value, comments, is_array_of_tables)?;
        Ok(leading)
    }

    /// Parse a key-value pair; `leading` are the comments right above it
    fn parse_key_value_pair(&mut self, leading: Vec<Comment>) -> Result<TableEntry> {
        let mut comments = Comments::new();
        comments.before = leading;

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
            // Keywords and digit-only names are valid bare keys, as in TOML
            TokenKind::Bool(_)
            | TokenKind::Null
            | TokenKind::EnvFunc
            | TokenKind::Include
            | TokenKind::Integer { .. } => Ok(KeySegment {
                name: token.text.to_string(),
                quoted: false,
                quote_style: None,
            }),
            // A bare key such as `2024-01-01` lexes as a date
            TokenKind::DateTime { raw, .. }
                if raw.bytes().all(|b| b.is_ascii_digit() || b == b'-') =>
            {
                Ok(KeySegment {
                    name: token.text.to_string(),
                    quoted: false,
                    quote_style: None,
                })
            }
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
            // `inf` and `nan` are float literals in value position (TOML)
            TokenKind::Identifier(name @ ("inf" | "nan")) => {
                let value = if *name == "inf" {
                    f64::INFINITY
                } else {
                    f64::NAN
                };
                let raw = name.to_string();
                let token = self.advance()?;
                Ok(AstNode::new(AstValue::Float { value, raw }, token.span))
            }
            TokenKind::Identifier(name) => Err(NomlError::parse_with_suggestion(
                format!("Unexpected identifier '{name}' where a value was expected"),
                token.span.start_line,
                token.span.start_column,
                format!("String values must be quoted: \"{name}\""),
            )),

            // Literals
            TokenKind::String { .. } => self.parse_string_value(),
            TokenKind::Integer { .. } => self.parse_integer_value(),
            TokenKind::Float { .. } => self.parse_float_value(),
            TokenKind::Bool(_) => self.parse_bool_value(),
            TokenKind::DateTime { .. } => {
                let token = self.advance()?;
                match token.kind {
                    TokenKind::DateTime { value, raw } => Ok(AstNode::new(
                        AstValue::DateTime {
                            value,
                            raw: raw.to_string(),
                        },
                        token.span,
                    )),
                    _ => unreachable!("checked by the outer match"),
                }
            }
            TokenKind::Null => self.parse_null_value(),

            // Collections
            TokenKind::LeftBracket => self.nested(Self::parse_array),
            TokenKind::LeftBrace => self.nested(Self::parse_inline_table),

            // Functions and special constructs
            TokenKind::EnvFunc => self.nested(Self::parse_env_function),
            TokenKind::At => self.nested(Self::parse_native_type),
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

    /// Run a nested parse step, enforcing [`MAX_NESTING_DEPTH`]
    fn nested(&mut self, step: fn(&mut Self) -> Result<AstNode>) -> Result<AstNode> {
        if self.depth >= MAX_NESTING_DEPTH {
            let (line, column) = (self.current_line(), self.current_column());
            return Err(NomlError::parse(
                format!("Nesting is deeper than the limit of {MAX_NESTING_DEPTH} levels"),
                line,
                column,
            ));
        }
        self.depth += 1;
        let result = step(self);
        self.depth -= 1;
        result
    }

    /// Parse a string value
    fn parse_string_value(&mut self) -> Result<AstNode> {
        let token = self.advance()?;

        if let TokenKind::String {
            ref value,
            ref style,
        } = token.kind
        {
            // Only basic (double-quoted) strings process escapes
            let has_escapes = matches!(
                style,
                LexerStringStyle::Double | LexerStringStyle::TripleDouble
            ) && token.text.contains('\\');

            let ast_value = AstValue::String {
                value: value.clone(),
                style: convert_string_style(style),
                has_escapes,
            };
            Ok(AstNode::new(ast_value, token.span))
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
            Ok(AstNode::new(ast_value, token.span))
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
            Ok(AstNode::new(ast_value, token.span))
        } else {
            unreachable!("Expected float token")
        }
    }

    /// Parse a boolean value
    fn parse_bool_value(&mut self) -> Result<AstNode> {
        let token = self.advance()?;

        if let TokenKind::Bool(value) = token.kind {
            let ast_value = AstValue::Bool(value);
            Ok(AstNode::new(ast_value, token.span))
        } else {
            unreachable!("Expected bool token")
        }
    }

    /// Parse a null value
    fn parse_null_value(&mut self) -> Result<AstNode> {
        let token = self.advance()?;
        let ast_value = AstValue::Null;
        Ok(AstNode::new(ast_value, token.span))
    }

    /// Parse an array
    fn parse_array(&mut self) -> Result<AstNode> {
        let start_span = self.current_span();

        // Consume '['
        self.consume_token(&TokenKind::LeftBracket, "Expected '['")?;

        let mut elements: Vec<AstNode> = Vec::new();
        let mut trailing_comma = false;
        let mut comments = Comments::new();

        loop {
            let leading = self.take_comments();

            // Closing bracket (empty array, or after a trailing comma)
            if self.check_token(&TokenKind::RightBracket) {
                comments.after = leading;
                break;
            }

            let mut element = self.parse_value()?;
            element.comments.before = leading;

            if self.match_token(&TokenKind::Comma) {
                trailing_comma = true;
                if let Some(comment) = self.parse_inline_comment()? {
                    element.comments.set_inline(comment);
                }
                elements.push(element);
            } else {
                trailing_comma = false;
                if let Some(comment) = self.parse_inline_comment()? {
                    element.comments.set_inline(comment);
                }
                elements.push(element);
                comments.after = self.take_comments();
                if !self.check_token(&TokenKind::RightBracket) {
                    return Err(NomlError::parse(
                        "Expected ',' or ']' in array",
                        self.current_line(),
                        self.current_column(),
                    ));
                }
                break;
            }
        }
        if elements.is_empty() {
            trailing_comma = false;
        }

        // Consume ']'
        let end_line = self.current_line();
        self.consume_token(&TokenKind::RightBracket, "Expected ']'")?;

        let end_span = self.current_span();
        let span = start_span.merge(&end_span);

        let ast_value = AstValue::Array {
            elements,
            multiline: end_line != start_span.start_line,
            trailing_comma,
        };

        Ok(AstNode::with_comments(ast_value, span, comments))
    }

    /// Parse an inline table
    fn parse_inline_table(&mut self) -> Result<AstNode> {
        let start_span = self.current_span();

        // Consume '{'
        self.consume_token(&TokenKind::LeftBrace, "Expected '{'")?;

        let mut entries = Vec::new();

        loop {
            let leading = self.take_comments();
            if self.check_token(&TokenKind::RightBrace) {
                break;
            }

            let entry = self.parse_key_value_pair(leading)?;
            entries.push(entry);

            // Check for comma or end
            if self.match_token(&TokenKind::Comma) {
                continue;
            }
            self.take_comments();
            if self.check_token(&TokenKind::RightBrace) {
                break;
            }
            return Err(NomlError::parse(
                "Expected ',' or '}' in inline table",
                self.current_line(),
                self.current_column(),
            ));
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

        // Parse the path: dot-separated key names and array indices. Quoted
        // segments keep their quotes so keys containing dots stay unambiguous.
        let mut path = String::new();
        loop {
            let token = self.advance()?;
            match &token.kind {
                TokenKind::Identifier(_)
                | TokenKind::Integer { .. }
                | TokenKind::Bool(_)
                | TokenKind::Null
                | TokenKind::EnvFunc
                | TokenKind::Include => path.push_str(token.text),
                // `items.1.2` lexes `1.2` as a float; it is two index segments
                TokenKind::Float { raw, .. }
                    if raw.bytes().all(|b| b.is_ascii_digit() || b == b'.') =>
                {
                    path.push_str(raw)
                }
                TokenKind::String { value, .. } => {
                    // Quoted segment; `\` and `"` are escaped so the path
                    // reads back the same way
                    path.push('"');
                    for c in value.chars() {
                        if c == '"' || c == '\\' {
                            path.push('\\');
                        }
                        path.push(c);
                    }
                    path.push('"');
                }
                _ => {
                    return Err(NomlError::parse_with_suggestion(
                        format!(
                            "Expected a key name in interpolation path, found {}",
                            token.kind
                        ),
                        token.span.start_line,
                        token.span.start_column,
                        "Interpolation paths are dotted keys, e.g. '${server.host}'",
                    ));
                }
            }
            if self.match_token(&TokenKind::Dot) {
                path.push('.');
            } else {
                break;
            }
        }

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

        // `include "path"` or `include("path")`
        let parenthesized = self.match_token(&TokenKind::LeftParen);

        // Parse the path string
        let token = self.advance()?;
        let path = match token.kind {
            TokenKind::String { value, .. } => value,
            other => {
                return Err(NomlError::parse(
                    format!("Expected a quoted file path after 'include', found {other}"),
                    token.span.start_line,
                    token.span.start_column,
                ));
            }
        };
        if parenthesized {
            self.consume_token(&TokenKind::RightParen, "Expected ')' after include path")?;
        }

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
    #[inline]
    fn peek(&self) -> Result<&Token<'_>> {
        self.tokens
            .get(self.pos)
            .ok_or_else(|| NomlError::parse("Unexpected end of input", 1, 1))
    }

    /// Advance to next token
    #[inline]
    fn advance(&mut self) -> Result<Token<'a>> {
        if self.is_at_end() {
            return Err(NomlError::parse(
                "Unexpected end of input",
                self.current_line(),
                self.current_column(),
            ));
        }
        // Tokens are never revisited once consumed, so move the token out
        // instead of cloning it (a clone copies every string and comment).
        let slot = &mut self.tokens[self.pos];
        let placeholder = Token {
            kind: TokenKind::Whitespace,
            span: slot.span,
            text: slot.text,
        };
        let token = std::mem::replace(slot, placeholder);
        self.pos += 1;
        Ok(token)
    }

    /// Check if current token matches given kind
    #[inline]
    fn check_token(&self, kind: &TokenKind) -> bool {
        if let Ok(token) = self.peek() {
            std::mem::discriminant(&token.kind) == std::mem::discriminant(kind)
        } else {
            false
        }
    }

    /// Match and consume token if it matches
    #[inline]
    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check_token(kind) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Consume token or return error
    fn consume_token(&mut self, kind: &TokenKind, message: &str) -> Result<&Token<'_>> {
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

    /// Get current position for span calculation
    fn current_span(&self) -> Span {
        if let Ok(token) = self.peek() {
            token.span
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

    /// Take the comment tokens at the current position
    fn take_comments(&mut self) -> Vec<Comment> {
        let mut comments = Vec::new();
        while let Some(token) = self.tokens.get(self.pos) {
            match &token.kind {
                TokenKind::Comment { text } => {
                    comments.push(Comment {
                        text: text.clone(),
                        span: token.span,
                        style: CommentStyle::Line,
                    });
                    self.pos += 1;
                }
                TokenKind::Whitespace | TokenKind::Newline => self.pos += 1,
                _ => break,
            }
        }
        comments
    }

    /// Parse a comment on the same line as the previous token
    fn parse_inline_comment(&mut self) -> Result<Option<Comment>> {
        let previous_line = match self.pos.checked_sub(1).and_then(|i| self.tokens.get(i)) {
            Some(token) => token.span.end_line,
            None => return Ok(None),
        };

        if let Some(token) = self.tokens.get(self.pos) {
            if let TokenKind::Comment { text } = &token.kind {
                if token.span.start_line == previous_line {
                    let comment = Comment {
                        text: text.clone(),
                        span: token.span,
                        style: CommentStyle::Line,
                    };
                    self.pos += 1;
                    return Ok(Some(comment));
                }
            }
        }

        Ok(None)
    }
}

/// True for an array built from `[[section]]` headers
fn is_table_array(node: &AstNode) -> bool {
    matches!(&node.value, AstValue::Array { elements, .. }
        if !elements.is_empty()
            && elements
                .iter()
                .all(|e| matches!(e.value, AstValue::Table { inline: false, .. })))
}

/// Place a `[section]` or `[[section]]` in `entries`.
///
/// A section whose key extends an earlier `[[array]]` key belongs to the
/// array's current last element (TOML rule), so it is stored inside that
/// element. Deciding this while parsing keeps later `[[array]]` headers from
/// changing which element an earlier section refers to.
fn attach_section(
    entries: &mut Vec<TableEntry>,
    key: Key,
    table: AstNode,
    comments: Comments,
    is_array: bool,
) -> Result<()> {
    // Most recent array-of-tables whose key is a strict prefix of this key
    let parent = entries.iter_mut().rev().find(|e| {
        e.key.segments.len() < key.segments.len()
            && key.segments.starts_with(&e.key.segments)
            && is_table_array(&e.value)
    });
    if let Some(parent) = parent {
        let prefix = parent.key.segments.len();
        if let AstValue::Array { elements, .. } = &mut parent.value.value {
            if let Some(AstValue::Table { entries: inner, .. }) =
                elements.last_mut().map(|e| &mut e.value)
            {
                let rest = Key::dotted(key.segments[prefix..].to_vec(), key.span);
                return attach_section(inner, rest, table, comments, is_array);
            }
        }
    }

    if is_array {
        // Add to the existing array of tables, or start one
        if let Some(existing) = entries.iter_mut().find(|e| e.key == key) {
            if is_table_array(&existing.value) {
                if let AstValue::Array { elements, .. } = &mut existing.value.value {
                    elements.push(table);
                }
                return Ok(());
            }
            return Err(NomlError::parse(
                format!("'{key}' is already defined and is not an array of tables"),
                key.span.start_line,
                key.span.start_column,
            ));
        }
        let span = table.span;
        entries.push(TableEntry {
            key,
            value: AstNode::new(
                AstValue::Array {
                    elements: vec![table],
                    multiline: true,
                    trailing_comma: false,
                },
                span,
            ),
            comments,
        });
    } else {
        entries.push(TableEntry {
            key,
            value: table,
            comments,
        });
    }
    Ok(())
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
