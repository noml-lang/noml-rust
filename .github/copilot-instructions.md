# NOML Rust - AI Coding Agent Instructions

## Project Overview

NOML (Nested Object Markup Language) is a modern configuration language that extends TOML with dynamic features. This Rust implementation provides a high-performance parser and resolver with full source fidelity preservation.

## Architecture Components

### Core Modules
- **`src/lib.rs`** - Main public API with convenience functions (`parse()`, `parse_from_file()`, `validate()`)
- **`src/parser/`** - Hand-written recursive descent parser with zero-copy lexer
  - `ast.rs` - AST nodes with source location preservation (`Document`, `AstNode`, `Span`)
  - `lexer.rs` - Zero-copy tokenizer with `Token` and `TokenKind` 
  - `grammar.rs` - Core parsing logic (`NomlParser`)
- **`src/value/`** - Value system with TOML-compatible types plus extensions (`Value` enum)
- **`src/resolver.rs`** - Dynamic feature resolution (env vars, includes, interpolation, native types)
- **`src/config/`** - High-level configuration management API
- **`src/main.rs`** - CLI tool with validate/parse commands

### Key Data Flow
1. **Parse**: Text → Tokens (lexer) → AST (parser) 
2. **Resolve**: AST → Value (resolver handles env(), includes, ${interpolation}, @native())
3. **Access**: Value provides typed accessors (`as_string()`, `get("path.to.key")`)

## Development Patterns

### Error Handling
- Custom `NomlError` type with source location context
- `Result<T>` alias for `std::result::Result<T, NomlError>`
- Parser preserves exact error positions for detailed reporting

### Value Access Pattern
```rust
// Standard pattern for safe value extraction
let port = config.get("server.port")?.as_integer()?;
let name = config.get("app.name")?.as_string()?;
```

### Testing Structure
- **Unit tests**: In-module `#[cfg(test)]` blocks in `src/`
- **Integration tests**: `tests/integration_tests.rs` (789 lines) - comprehensive feature coverage
- **Benchmarks**: `benches/parser_benchmarks.rs` using Criterion framework

### Build Commands
```bash
# Core development workflow
cargo test                    # Run all tests
cargo bench                   # Performance benchmarks  
cargo build --release        # Optimized build (opt-level=3, LTO enabled)
cargo run -- validate file.noml  # CLI validation
cargo run -- parse file.noml     # CLI parsing/display
```

## NOML-Specific Features

### Dynamic Resolution (Resolver Pattern)
- **Environment vars**: `env("VAR_NAME", "default")` 
- **File includes**: `include "path/to/file.noml"`
- **Variable interpolation**: `"Hello ${name}!"` 
- **Native types**: `@size("10MB")`, `@duration("30s")`, `@url("https://...")`

### AST Preservation
- Full source fidelity with comments, whitespace, formatting preserved
- `Span` tracks exact byte positions and line/column numbers
- Enables round-trip editing and precise error reporting

### Performance Optimizations
- Zero-copy lexer using string slices
- Hand-written recursive descent parser (no parser generator)
- Release profile: `opt-level = 3`, `lto = true`, `codegen-units = 1`

## Dependencies & Features
- **Required**: `indexmap` (preserves key order), `serde`, `tempfile`, `thiserror`
- **Optional**: `chrono` feature for DateTime support
- **Dev**: `criterion` (benchmarks), `tokio-test`

## Convention Notes
- Use `BTreeMap` for deterministic ordering in tables
- Preserve source information in AST for tooling support  
- CLI tool in `main.rs` follows standard Unix patterns (validate/parse commands)
- All dynamic features resolved through `Resolver` with configurable behavior