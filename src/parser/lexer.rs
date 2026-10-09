//! # NOML Lexer
//!
//! High-performance tokenizer for NOML using zero-copy string slicing.
//! This lexer is designed for maximum speed while preserving all source
//! information needed for perfect round-trip serialization.

use crate::datetime::{self, Datetime};
use crate::error::{NomlError, Result};
use crate::parser::Span;
use std::fmt;

/// A token in the NOML source code
#[derive(Debug, Clone, PartialEq)]
pub struct Token<'a> {
    /// Token type and associated data
    pub kind: TokenKind<'a>,
    /// Source location of this token
    pub span: Span,
    /// Original source text (for perfect reconstruction)
    pub text: &'a str,
}

/// Token types with associated data
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TokenKind<'a> {
    // Literals
    /// String literal with quote style and processed value
    String {
        /// The processed string value (escapes resolved)
        value: String,
        /// Original quote style
        style: StringStyle,
    },

    /// Integer literal with parsed value and original representation
    Integer {
        /// Parsed integer value
        value: i64,
        /// Original text (preserves hex, octal, binary formats)
        raw: &'a str,
    },

    /// Float literal with parsed value and original representation
    Float {
        /// Parsed float value
        value: f64,
        /// Original text (preserves format)
        raw: &'a str,
    },

    /// Boolean literal
    Bool(bool),

    /// Date and/or time literal such as `1979-05-27T07:32:00Z`
    DateTime {
        /// Parsed value
        value: Datetime,
        /// Original text
        raw: &'a str,
    },

    /// Null literal
    Null,

    // Identifiers and keywords
    /// Bare identifier (unquoted key names, function names)
    Identifier(&'a str),

    /// Environment variable function
    EnvFunc,

    /// Include/import directive
    Include,

    // Symbols and operators
    /// = (assignment)
    Equals,

    /// . (dot for key paths)
    Dot,

    /// , (comma separator)
    Comma,

    /// [ (left bracket - array start or table header start)
    LeftBracket,

    /// ] (right bracket - array end or table header end)
    RightBracket,

    /// { (left brace - inline table start)
    LeftBrace,

    /// } (right brace - inline table end)
    RightBrace,

    /// ( (left parenthesis - function call start)
    LeftParen,

    /// ) (right parenthesis - function call end)
    RightParen,

    // String interpolation
    /// ${ (start of interpolation)
    InterpolationStart,

    /// @ (native type constructor prefix)
    At,

    // Whitespace and comments
    /// Line comment starting with #
    Comment {
        /// Comment text without the # prefix
        text: String,
    },

    /// Whitespace (spaces, tabs)
    Whitespace,

    /// Newline characters
    Newline,

    // Special tokens
    /// End of file
    Eof,

    /// Invalid/unrecognized character
    Invalid(char),
}

/// String quoting styles
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
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
        /// Number of `#` characters used in the raw string delimiter
        hashes: usize,
    },
}

/// High-performance lexer with zero-copy tokenization
pub struct Lexer<'a> {
    /// Input source text
    input: &'a str,
    /// Current byte position
    pos: usize,
    /// Current line number (1-indexed)
    line: usize,
    /// Current column number (1-indexed, in characters)
    column: usize,
    /// Start position of current token
    token_start: usize,
    /// Start line of current token
    token_start_line: usize,
    /// Start column of current token
    token_start_column: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given input
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            line: 1,
            column: 1,
            token_start: 0,
            token_start_line: 1,
            token_start_column: 1,
        }
    }

    /// Get the next token from the input
    pub fn next_token(&mut self) -> Result<Token<'a>> {
        self.start_token();

        if self.is_eof() {
            return Ok(self.make_token(TokenKind::Eof));
        }

        let ch = self.current_char();
        match ch {
            // Whitespace (spaces, tabs, carriage returns)
            ' ' | '\t' | '\r' => {
                while !self.is_eof() && matches!(self.current_char(), ' ' | '\t' | '\r') {
                    self.advance();
                }
                Ok(self.make_token(TokenKind::Whitespace))
            }
            // Newline
            '\n' => {
                self.advance();
                Ok(self.make_token(TokenKind::Newline))
            }
            // Comments
            '#' => self.lex_comment(),

            // Strings
            '"' => {
                if self.rest().starts_with("\"\"\"") {
                    self.lex_basic_string(true)
                } else {
                    self.lex_basic_string(false)
                }
            }
            '\'' => {
                if self.rest().starts_with("'''") {
                    self.lex_literal_string(true)
                } else {
                    self.lex_literal_string(false)
                }
            }
            'r' if self.peek_char() == Some('"') || self.peek_char() == Some('#') => {
                self.lex_raw_string()
            }

            // Signed special floats: +inf, -inf, +nan, -nan
            '+' | '-' if self.signed_special_float() => {
                self.advance(); // sign
                for _ in 0..3 {
                    self.advance();
                }
                let raw = &self.input[self.token_start..self.pos];
                let value = match raw {
                    "+inf" => f64::INFINITY,
                    "-inf" => f64::NEG_INFINITY,
                    _ => f64::NAN,
                };
                Ok(self.make_token(TokenKind::Float { value, raw }))
            }

            // Dates and times, then numbers
            '0'..='9' => match datetime::lex(self.rest()) {
                Ok(Some((value, len))) => {
                    // Date/time literals are ASCII, so one byte is one column
                    self.pos += len;
                    self.column += len;
                    let raw = &self.input[self.token_start..self.pos];
                    Ok(self.make_token(TokenKind::DateTime { value, raw }))
                }
                Ok(None) => self.lex_number(),
                Err(message) => Err(NomlError::parse(
                    format!("{message}: {}", self.datetime_text()),
                    self.line,
                    self.column,
                )),
            },
            '-' | '+' if matches!(self.peek_char(), Some('0'..='9')) => self.lex_number(),

            // Symbols
            '=' => self.single(TokenKind::Equals),
            '.' => self.single(TokenKind::Dot),
            ',' => self.single(TokenKind::Comma),
            '[' => self.single(TokenKind::LeftBracket),
            ']' => self.single(TokenKind::RightBracket),
            '{' => self.single(TokenKind::LeftBrace),
            '}' => self.single(TokenKind::RightBrace),
            '(' => self.single(TokenKind::LeftParen),
            ')' => self.single(TokenKind::RightParen),
            '@' => self.single(TokenKind::At),

            // Interpolation
            '$' if self.peek_char() == Some('{') => {
                self.advance(); // $
                self.advance(); // {
                Ok(self.make_token(TokenKind::InterpolationStart))
            }
            // Identifiers (bare keys, function names)
            ch if ch.is_ascii_alphabetic() || ch == '_' => self.lex_identifier(),

            // Unknown/invalid character
            ch => {
                self.advance();
                Ok(self.make_token(TokenKind::Invalid(ch)))
            }
        }
    }

    /// Tokenize the entire input into a vector of tokens.
    ///
    /// Whitespace and newline tokens are dropped; comments are kept so the
    /// parser can attach them to the surrounding entries.
    pub fn tokenize(&mut self) -> Result<Vec<Token<'a>>> {
        // Rough guess: one significant token per four bytes of input.
        let mut tokens = Vec::with_capacity(self.input.len() / 4 + 1);

        loop {
            let token = self.next_token()?;
            let is_eof = matches!(token.kind, TokenKind::Eof);

            match token.kind {
                TokenKind::Whitespace | TokenKind::Newline => {}
                _ => tokens.push(token),
            }

            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }

    // Helper methods

    /// Start tracking a new token
    #[inline]
    fn start_token(&mut self) {
        self.token_start = self.pos;
        self.token_start_line = self.line;
        self.token_start_column = self.column;
    }

    /// Create a token with the current span
    #[inline]
    fn make_token(&self, kind: TokenKind<'a>) -> Token<'a> {
        Token {
            kind,
            span: Span::new(
                self.token_start,
                self.pos,
                self.token_start_line,
                self.token_start_column,
                self.line,
                self.column,
            ),
            text: &self.input[self.token_start..self.pos],
        }
    }

    /// Consume one character and emit a token of the given kind
    #[inline]
    fn single(&mut self, kind: TokenKind<'a>) -> Result<Token<'a>> {
        self.advance();
        Ok(self.make_token(kind))
    }

    /// The unconsumed part of the input
    #[inline]
    fn rest(&self) -> &'a str {
        &self.input[self.pos..]
    }

    /// Get the current character without advancing ('\0' at end of input)
    #[inline]
    fn current_char(&self) -> char {
        match self.input.as_bytes().get(self.pos) {
            Some(&b) if b < 0x80 => b as char,
            Some(_) => self.rest().chars().next().unwrap_or('\0'),
            None => '\0',
        }
    }

    /// Peek at the character after the current one
    #[inline]
    fn peek_char(&self) -> Option<char> {
        let mut chars = self.rest().chars();
        chars.next();
        chars.next()
    }

    /// Advance by one character
    #[inline]
    fn advance(&mut self) -> Option<char> {
        let ch = self.rest().chars().next()?;
        self.pos += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    /// Check if we're at end of file
    #[inline]
    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// The date/time-looking text at the current position, for error messages
    fn datetime_text(&self) -> &'a str {
        let rest = self.rest();
        let end = rest
            .find(|c: char| c.is_whitespace() || matches!(c, ',' | ']' | '}' | '#'))
            .unwrap_or(rest.len());
        &rest[..end]
    }

    /// True if the input at the current position is `+inf`, `-inf`, `+nan` or `-nan`
    fn signed_special_float(&self) -> bool {
        let rest = &self.rest()[1..];
        (rest.starts_with("inf") || rest.starts_with("nan"))
            && !rest[3..]
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    /// Lex a comment starting with #
    fn lex_comment(&mut self) -> Result<Token<'a>> {
        self.advance(); // Skip #

        let start_pos = self.pos;
        let len = self.rest().find('\n').unwrap_or(self.rest().len());
        let comment_slice = &self.input[start_pos..start_pos + len];
        // Comments never contain newlines, so only the column moves.
        self.column += comment_slice.chars().count();
        self.pos += len;

        let text = comment_slice.trim_start().to_string();

        Ok(self.make_token(TokenKind::Comment { text }))
    }

    /// Error for a string that runs into the end of the input
    fn unterminated(&self, what: &str) -> NomlError {
        NomlError::parse(
            format!("Unterminated {what} (opened here)"),
            self.token_start_line,
            self.token_start_column,
        )
    }

    /// Skip a newline that directly follows the opening delimiter of a
    /// multi-line string (TOML trims it).
    fn skip_leading_newline(&mut self) {
        if self.rest().starts_with("\r\n") {
            self.advance();
            self.advance();
        } else if self.rest().starts_with('\n') {
            self.advance();
        }
    }

    /// Lex a basic (double-quoted) string, single or multi-line.
    ///
    /// Escapes are resolved here. `${...}` is kept as plain text; the resolver
    /// expands it later.
    fn lex_basic_string(&mut self, multiline: bool) -> Result<Token<'a>> {
        let delim = if multiline { 3 } else { 1 };
        for _ in 0..delim {
            self.advance();
        }
        if multiline {
            self.skip_leading_newline();
        }

        let mut value = String::new();
        let mut seg_start = self.pos;

        loop {
            if self.is_eof() {
                return Err(self.unterminated("string literal"));
            }
            let ch = self.current_char();

            if ch == '"' {
                if !multiline {
                    value.push_str(&self.input[seg_start..self.pos]);
                    self.advance();
                    break;
                }
                if self.rest().starts_with("\"\"\"") {
                    // Up to two quotes may sit right before the closing delimiter.
                    let run = self
                        .rest()
                        .bytes()
                        .take_while(|b| *b == b'"')
                        .count()
                        .min(5);
                    value.push_str(&self.input[seg_start..self.pos]);
                    for _ in 3..run {
                        value.push('"');
                    }
                    for _ in 0..run {
                        self.advance();
                    }
                    break;
                }
                self.advance();
                continue;
            }

            if ch == '\\' {
                value.push_str(&self.input[seg_start..self.pos]);
                self.advance(); // backslash

                if multiline && self.at_line_ending_backslash() {
                    // Line-ending backslash: drop the newline and leading
                    // whitespace of the following lines.
                    while !self.is_eof() && matches!(self.current_char(), ' ' | '\t' | '\r' | '\n')
                    {
                        self.advance();
                    }
                } else {
                    self.lex_escape(&mut value)?;
                }
                seg_start = self.pos;
                continue;
            }

            self.advance();
        }

        let style = if multiline {
            StringStyle::TripleDouble
        } else {
            StringStyle::Double
        };
        Ok(self.make_token(TokenKind::String { value, style }))
    }

    /// After a backslash in a multi-line basic string: is the rest of the line blank?
    fn at_line_ending_backslash(&self) -> bool {
        for b in self.rest().bytes() {
            match b {
                b' ' | b'\t' | b'\r' => continue,
                b'\n' => return true,
                _ => return false,
            }
        }
        false
    }

    /// Resolve one escape sequence (the backslash is already consumed)
    fn lex_escape(&mut self, value: &mut String) -> Result<()> {
        if self.is_eof() {
            return Err(NomlError::parse(
                "Unterminated string escape",
                self.line,
                self.column,
            ));
        }
        let (line, column) = (self.line, self.column.saturating_sub(1));
        let ch = self.current_char();
        match ch {
            'n' => value.push('\n'),
            't' => value.push('\t'),
            'r' => value.push('\r'),
            'b' => value.push('\u{8}'),
            'f' => value.push('\u{c}'),
            'e' => value.push('\u{1b}'),
            '\\' => value.push('\\'),
            '"' => value.push('"'),
            '\'' => value.push('\''),
            '0' => value.push('\0'),
            'u' | 'U' => {
                self.advance();
                let digits = if ch == 'u' && self.current_char() == '{' {
                    // \u{1F600}
                    self.advance();
                    let start = self.pos;
                    let len = self.rest().find('}').ok_or_else(|| {
                        NomlError::parse("Unterminated unicode escape", line, column)
                    })?;
                    let digits = &self.input[start..start + len];
                    if digits.is_empty()
                        || digits.len() > 6
                        || !digits.bytes().all(|b| b.is_ascii_hexdigit())
                    {
                        return Err(NomlError::parse(
                            format!("Invalid unicode escape: \\u{{{digits}}}"),
                            line,
                            column,
                        ));
                    }
                    for _ in 0..=digits.chars().count() {
                        self.advance(); // digits and the closing brace
                    }
                    digits
                } else {
                    // \uXXXX or \UXXXXXXXX
                    let want = if ch == 'u' { 4 } else { 8 };
                    let start = self.pos;
                    let digits = self.rest().get(..want).unwrap_or("");
                    if digits.len() != want || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
                        return Err(NomlError::parse(
                            format!("Invalid unicode escape: \\{ch} needs {want} hex digits"),
                            line,
                            column,
                        ));
                    }
                    for _ in 0..want {
                        self.advance();
                    }
                    &self.input[start..start + want]
                };
                let code = u32::from_str_radix(digits, 16).map_err(|_| {
                    NomlError::parse(
                        format!("Invalid unicode escape value: {digits}"),
                        line,
                        column,
                    )
                })?;
                let c = char::from_u32(code).ok_or_else(|| {
                    NomlError::parse(
                        format!("Invalid unicode code point: U+{code:X}"),
                        line,
                        column,
                    )
                })?;
                value.push(c);
                return Ok(());
            }
            other => {
                return Err(NomlError::parse(
                    format!("Invalid escape sequence: \\{other}"),
                    line,
                    column,
                ));
            }
        }
        self.advance();
        Ok(())
    }

    /// Lex a literal (single-quoted) string, single or multi-line.
    ///
    /// Literal strings follow TOML: no escapes, the content is taken as written.
    fn lex_literal_string(&mut self, multiline: bool) -> Result<Token<'a>> {
        let delim = if multiline { 3 } else { 1 };
        for _ in 0..delim {
            self.advance();
        }
        if multiline {
            self.skip_leading_newline();
        }

        let start = self.pos;
        let value;
        loop {
            if self.is_eof() {
                return Err(self.unterminated("string literal"));
            }
            if self.current_char() == '\'' {
                if !multiline {
                    value = self.input[start..self.pos].to_string();
                    self.advance();
                    break;
                }
                if self.rest().starts_with("'''") {
                    let run = self
                        .rest()
                        .bytes()
                        .take_while(|b| *b == b'\'')
                        .count()
                        .min(5);
                    let mut v = self.input[start..self.pos].to_string();
                    for _ in 3..run {
                        v.push('\'');
                    }
                    for _ in 0..run {
                        self.advance();
                    }
                    value = v;
                    break;
                }
            }
            self.advance();
        }

        let style = if multiline {
            StringStyle::TripleSingle
        } else {
            StringStyle::Single
        };
        Ok(self.make_token(TokenKind::String { value, style }))
    }

    /// Lex a raw string literal: r"..." or r#"..."#
    fn lex_raw_string(&mut self) -> Result<Token<'a>> {
        self.advance(); // Skip 'r'

        // Count hashes
        let mut hashes = 0;
        while self.current_char() == '#' {
            hashes += 1;
            self.advance();
        }

        // Expect opening quote
        if self.current_char() != '"' {
            return Err(NomlError::parse(
                "Expected '\"' after raw string prefix",
                self.line,
                self.column,
            ));
        }
        self.advance(); // Skip opening quote

        let content_start = self.pos;

        // Find the closing sequence: '"' followed by the same number of '#'
        while !self.is_eof() {
            if self.current_char() == '"' {
                let after = &self.input.as_bytes()[self.pos + 1..];
                if after.len() >= hashes && after[..hashes].iter().all(|b| *b == b'#') {
                    let value = self.input[content_start..self.pos].to_string();
                    self.advance(); // Skip closing quote
                    for _ in 0..hashes {
                        self.advance();
                    }
                    return Ok(self.make_token(TokenKind::String {
                        value,
                        style: StringStyle::Raw { hashes },
                    }));
                }
            }
            self.advance();
        }

        Err(self.unterminated("raw string"))
    }

    /// Lex a number (integer or float)
    fn lex_number(&mut self) -> Result<Token<'a>> {
        let start_pos = self.pos;
        let sign = self.current_char();
        let sign_len = if sign == '-' || sign == '+' {
            self.advance();
            1
        } else {
            0
        };
        let is_negative = sign == '-';

        let mut is_float = false;
        let mut base = 10;

        // Check for hex, octal, or binary prefix
        if self.current_char() == '0' {
            match self.peek_char() {
                Some('x') | Some('X') => base = 16,
                Some('o') | Some('O') => base = 8,
                Some('b') | Some('B') => base = 2,
                _ => {}
            }
            if base != 10 {
                self.advance(); // 0
                self.advance(); // x / o / b
            }
        }

        // Read digits
        while !self.is_eof() {
            let ch = self.current_char();
            if base == 10 && ch == '.' && !is_float {
                // Only a dot followed by a digit belongs to the number
                if !matches!(self.peek_char(), Some('0'..='9')) {
                    break;
                }
                is_float = true;
                self.advance();
            } else if base == 10 && (ch == 'e' || ch == 'E') {
                is_float = true;
                self.advance();
                if matches!(self.current_char(), '+' | '-') {
                    self.advance();
                }
            } else if ch == '_' || ch.is_digit(base) || (base == 10 && ch == '.') {
                // A second '.' is consumed so the error covers the whole literal
                self.advance();
            } else {
                break;
            }
        }

        let raw_text = &self.input[start_pos..self.pos];
        let invalid = |kind: &str| {
            NomlError::parse(
                format!("Invalid {kind} literal: {raw_text}"),
                self.token_start_line,
                self.token_start_column,
            )
        };
        let clean_text = if raw_text.contains('_') {
            std::borrow::Cow::Owned(raw_text.replace('_', ""))
        } else {
            std::borrow::Cow::Borrowed(raw_text)
        };

        if is_float {
            let value = clean_text.parse::<f64>().map_err(|_| invalid("float"))?;
            Ok(self.make_token(TokenKind::Float {
                value,
                raw: raw_text,
            }))
        } else {
            let value = if base == 10 {
                clean_text.parse::<i64>().map_err(|_| invalid("integer"))?
            } else {
                let digits = &clean_text[sign_len + 2..];
                // Parse the magnitude as u64 so i64::MIN in hex/octal/binary works.
                let magnitude =
                    u64::from_str_radix(digits, base).map_err(|_| invalid("integer"))?;
                if is_negative {
                    0i64.checked_sub_unsigned(magnitude)
                        .ok_or_else(|| invalid("integer"))?
                } else {
                    i64::try_from(magnitude).map_err(|_| invalid("integer"))?
                }
            };

            Ok(self.make_token(TokenKind::Integer {
                value,
                raw: raw_text,
            }))
        }
    }

    /// Lex an identifier or keyword
    fn lex_identifier(&mut self) -> Result<Token<'a>> {
        let start = self.pos;

        // First character is already validated as alphabetic or underscore
        self.advance();

        // Bare keys may contain letters, digits, '_' and '-' (as in TOML)
        while !self.is_eof() {
            let ch = self.current_char();
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                self.advance();
            } else {
                break;
            }
        }

        let text = &self.input[start..self.pos];

        // Check for keywords
        let kind = match text {
            "true" => TokenKind::Bool(true),
            "false" => TokenKind::Bool(false),
            "null" => TokenKind::Null,
            "env" => TokenKind::EnvFunc,
            "include" => TokenKind::Include,
            _ => TokenKind::Identifier(text),
        };

        Ok(self.make_token(kind))
    }
}

impl fmt::Display for TokenKind<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::String { value, .. } => write!(f, "\"{value}\""),
            TokenKind::Integer { value, .. } => write!(f, "{value}"),
            TokenKind::Float { value, .. } => write!(f, "{value}"),
            TokenKind::Bool(b) => write!(f, "{b}"),
            TokenKind::DateTime { raw, .. } => write!(f, "{raw}"),
            TokenKind::Null => write!(f, "null"),
            TokenKind::Identifier(name) => write!(f, "{name}"),
            TokenKind::EnvFunc => write!(f, "env"),
            TokenKind::Include => write!(f, "include"),
            TokenKind::Equals => write!(f, "="),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Comma => write!(f, ","),
            TokenKind::LeftBracket => write!(f, "["),
            TokenKind::RightBracket => write!(f, "]"),
            TokenKind::LeftBrace => write!(f, "{{"),
            TokenKind::RightBrace => write!(f, "}}"),
            TokenKind::LeftParen => write!(f, "("),
            TokenKind::RightParen => write!(f, ")"),
            TokenKind::InterpolationStart => write!(f, "${{"),
            TokenKind::At => write!(f, "@"),
            TokenKind::Comment { text } => write!(f, "# {text}"),
            TokenKind::Whitespace => write!(f, "<ws>"),
            TokenKind::Newline => write!(f, "<nl>"),
            TokenKind::Eof => write!(f, "<eof>"),
            TokenKind::Invalid(ch) => write!(f, "<invalid:{ch}>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize_string(input: &str) -> Result<Vec<Token<'_>>> {
        let mut lexer = Lexer::new(input);
        lexer.tokenize()
    }

    #[test]
    fn basic_tokens() {
        let input = r#"key = "value""#;
        let tokens = tokenize_string(input).unwrap();

        assert_eq!(tokens.len(), 4); // identifier, =, string, EOF
        assert!(matches!(tokens[0].kind, TokenKind::Identifier("key")));
        assert!(matches!(tokens[1].kind, TokenKind::Equals));
        if let TokenKind::String { value, .. } = &tokens[2].kind {
            assert_eq!(value, "value");
        } else {
            panic!("Expected string token");
        }
        assert!(matches!(tokens[3].kind, TokenKind::Eof));
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn numbers() {
        let input = "42 3.14 -10 0xFF 0o755 0b1010";
        let tokens = tokenize_string(input).unwrap();

        // Should have integer, float, negative integer, hex, octal, binary + EOF
        assert!(matches!(
            tokens[0].kind,
            TokenKind::Integer { value: 42, .. }
        ));
        let _ = matches!(tokens[1].kind, TokenKind::Float { value, .. } if (value - 3.14).abs() < f64::EPSILON);
        assert!(
            matches!(tokens[1].kind, TokenKind::Float { value, .. } if (value - 3.14).abs() < f64::EPSILON)
        );
        assert!(matches!(
            tokens[2].kind,
            TokenKind::Integer { value: -10, .. }
        ));
        assert!(matches!(
            tokens[3].kind,
            TokenKind::Integer { value: 255, .. }
        )); // 0xFF
        assert!(matches!(
            tokens[4].kind,
            TokenKind::Integer { value: 493, .. }
        )); // 0o755
        assert!(matches!(
            tokens[5].kind,
            TokenKind::Integer { value: 10, .. }
        )); // 0b1010
    }

    #[test]
    fn string_escapes() {
        let input = r#""hello\nworld\u{1F4A9}""#;
        let tokens = tokenize_string(input).unwrap();

        if let TokenKind::String { value, .. } = &tokens[0].kind {
            assert!(value.contains('\n'));
            assert!(value.contains('💩')); // Unicode poop emoji
        } else {
            panic!("Expected string token");
        }
    }

    #[test]
    fn raw_strings() {
        let input = r#"r"no\nescapes""#;
        let tokens = tokenize_string(input).unwrap();

        if let TokenKind::String { value, style } = &tokens[0].kind {
            assert_eq!(value, r"no\nescapes");
            assert!(matches!(style, StringStyle::Raw { hashes: 0 }));
        }

        let input2 = r##"r#"with"quotes"#"##;
        let tokens2 = tokenize_string(input2).unwrap();

        if let TokenKind::String { value, style } = &tokens2[0].kind {
            assert_eq!(value, r#"with"quotes"#);
            assert!(matches!(style, StringStyle::Raw { hashes: 1 }));
        }
    }

    #[test]
    fn comments() {
        let input = "# This is a comment\nkey = value # Inline comment";
        let tokens = tokenize_string(input).unwrap();

        // Should find both comments
        let comment_tokens: Vec<_> = tokens
            .iter()
            .filter_map(|t| {
                if let TokenKind::Comment { text } = &t.kind {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(comment_tokens.len(), 2);
        assert!(comment_tokens[0].contains("This is a comment"));
        assert!(comment_tokens[1].contains("Inline comment"));
    }

    #[test]
    fn complex_structures() {
        let input = r#"
        [server]
        host = "localhost"
        ports = [8080, 8081]
        config = { debug = true, timeout = @duration("30s") }
        "#;

        let tokens = tokenize_string(input).expect("Should tokenize successfully");

        // Should have various token types
        let has_bracket = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::LeftBracket));
        let has_brace = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::LeftBrace));
        let has_at = tokens.iter().any(|t| matches!(t.kind, TokenKind::At));

        assert!(has_bracket);
        assert!(has_brace);
        assert!(has_at);
    }

    #[test]
    fn interpolation() {
        // Inside a string, `${...}` stays part of the string value; the
        // resolver expands it.
        let input = r#"path = "${base}/logs""#;
        let tokens = tokenize_string(input).unwrap();
        assert_eq!(tokens.len(), 4);
        assert!(
            matches!(&tokens[2].kind, TokenKind::String { value, .. } if value == "${base}/logs")
        );
        assert!(!tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::InterpolationStart)));

        // A bare `${` outside a string is its own token
        let tokens = tokenize_string("path = ${base}").unwrap();
        assert!(matches!(tokens[2].kind, TokenKind::InterpolationStart));
    }

    fn string_value(input: &str) -> String {
        let tokens = tokenize_string(input).unwrap();
        match &tokens[0].kind {
            TokenKind::String { value, .. } => value.clone(),
            other => panic!("expected a string token, got {other:?}"),
        }
    }

    #[test]
    fn literal_strings_keep_backslashes() {
        assert_eq!(string_value(r"'C:\Users\noml'"), r"C:\Users\noml");
        assert_eq!(string_value(r"'${not_expanded}'"), "${not_expanded}");
    }

    #[test]
    fn multiline_strings() {
        assert_eq!(
            string_value("\"\"\"\nline one\nline two\"\"\""),
            "line one\nline two"
        );
        assert_eq!(string_value("'''\nraw \\n text'''"), "raw \\n text");
        assert_eq!(
            string_value("\"\"\"one \\\n      two\"\"\""),
            "one two",
            "line-ending backslash trims the newline and indentation"
        );
        assert_eq!(string_value("\"\"\"say \"hi\"\"\"\""), "say \"hi\"");
        assert!(tokenize_string("\"\"\"never closed").is_err());
    }

    #[test]
    fn toml_unicode_escapes() {
        assert_eq!(string_value(r#""\u00E9\U0001F600""#), "\u{e9}\u{1F600}");
        assert_eq!(string_value(r#""\u{e9}""#), "\u{e9}");
        assert!(tokenize_string(r#""\u{+41}""#).is_err());
        assert!(tokenize_string(r#""\u12""#).is_err());
        assert!(tokenize_string(r#""\uD800""#).is_err());
    }

    #[test]
    fn signed_numbers_and_special_floats() {
        let tokens = tokenize_string("+42 -0x10 +inf -inf nan -0b1").unwrap();
        assert!(matches!(
            tokens[0].kind,
            TokenKind::Integer { value: 42, .. }
        ));
        assert!(matches!(
            tokens[1].kind,
            TokenKind::Integer { value: -16, .. }
        ));
        assert!(matches!(tokens[2].kind, TokenKind::Float { value, .. } if value == f64::INFINITY));
        assert!(
            matches!(tokens[3].kind, TokenKind::Float { value, .. } if value == f64::NEG_INFINITY)
        );
        // Bare `nan`/`inf` are identifiers here; the parser turns them into floats
        assert!(matches!(tokens[4].kind, TokenKind::Identifier("nan")));
        assert!(matches!(
            tokens[5].kind,
            TokenKind::Integer { value: -1, .. }
        ));

        let tokens = tokenize_string("-0x8000000000000000").unwrap();
        assert!(matches!(
            tokens[0].kind,
            TokenKind::Integer {
                value: i64::MIN,
                ..
            }
        ));
        assert!(tokenize_string("0x8000000000000000").is_err());
    }

    #[test]
    fn dashed_identifiers() {
        let tokens = tokenize_string("server-name = 1").unwrap();
        assert!(matches!(
            tokens[0].kind,
            TokenKind::Identifier("server-name")
        ));
    }

    #[test]
    fn long_input_lexes_in_linear_time() {
        // 200k string values; the old lexer re-scanned from the start for
        // every character and took minutes on input this size.
        let source = "k = \"value with ünïcode\"\n".repeat(200_000);
        let started = std::time::Instant::now();
        let tokens = tokenize_string(&source).unwrap();
        assert_eq!(tokens.len(), 600_001);
        assert!(started.elapsed() < std::time::Duration::from_secs(10));
    }

    #[test]
    fn keywords() {
        let input = "true false null env include";
        let tokens = tokenize_string(input).unwrap();

        assert!(matches!(tokens[0].kind, TokenKind::Bool(true)));
        assert!(matches!(tokens[1].kind, TokenKind::Bool(false)));
        assert!(matches!(tokens[2].kind, TokenKind::Null));
        assert!(matches!(tokens[3].kind, TokenKind::EnvFunc));
        assert!(matches!(tokens[4].kind, TokenKind::Include));
    }

    #[test]
    fn span_information() {
        let input = "key = \"value\"";
        let tokens = tokenize_string(input).unwrap();

        // Check that spans are calculated correctly
        assert_eq!(tokens[0].span.start, 0); // "key" starts at beginning
        assert_eq!(tokens[0].span.end, 3); // "key" ends at position 3
        assert_eq!(tokens[1].span.start, 4); // "=" starts after space
        assert_eq!(tokens[2].span.start, 6); // String starts after space

        // Check line/column tracking
        assert_eq!(tokens[0].span.start_line, 1);
        assert_eq!(tokens[0].span.start_column, 1);
    }

    #[test]
    fn error_handling() {
        // Unterminated string
        let result = tokenize_string(r#""unterminated"#);
        assert!(result.is_err());

        // Invalid escape
        let result = tokenize_string(r#""\q""#);
        assert!(result.is_err());

        // Invalid unicode escape
        let result = tokenize_string(r#""\u{GGGG}""#);
        assert!(result.is_err());
    }
}
