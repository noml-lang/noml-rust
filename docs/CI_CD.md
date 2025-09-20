# CI/CD Documentation

This document describes the comprehensive CI/CD setup for NOML Rust.

## Overview

NOML uses GitHub Actions for continuous integration and deployment with the following workflows:

- **CI**: Comprehensive testing across platforms and Rust versions
- **Release**: Automated releases with binaries and crates.io publishing
- **Security**: Daily security audits and dependency checking
- **Benchmarks**: Performance regression detection

## Workflows

### Main CI Pipeline (`.github/workflows/ci.yml`)

Runs on every push and pull request to `main` and `develop` branches.

**Jobs:**
- **Check**: Basic compilation validation
- **Format**: Code formatting with `rustfmt`
- **Clippy**: Linting with comprehensive rules
- **Test**: Cross-platform testing matrix
- **Security**: Security audits with `cargo-audit`
- **Docs**: Documentation building and link checking
- **Benchmarks**: Performance testing (main branch only)
- **Miri**: Memory safety testing
- **Coverage**: Code coverage reporting
- **Integration**: Real-world configuration testing

**Test Matrix:**
- **Platforms**: Ubuntu, Windows, macOS (including ARM64)
- **Rust Versions**: Stable, Beta, MSRV (1.70.0)
- **Feature Combinations**: All features, no features, individual features

### Release Pipeline (`.github/workflows/release.yml`)

Triggered by version tags (e.g., `v1.0.0`).

**Process:**
1. Create GitHub release with changelog
2. Build release binaries for all platforms
3. Upload binaries as release assets
4. Publish to crates.io
5. Update documentation on GitHub Pages

**Supported Platforms:**
- Linux (x86_64-unknown-linux-gnu, x86_64-unknown-linux-musl)
- Windows (x86_64-pc-windows-msvc)
- macOS (x86_64-apple-darwin, aarch64-apple-darwin)

### Security Auditing (`.github/workflows/security.yml`)

Runs daily and on every push/PR.

**Checks:**
- Vulnerability scanning with `cargo-audit`
- Supply chain security with `cargo-vet`
- License compliance
- Dependency analysis

### Performance Monitoring (`.github/workflows/benchmark.yml`)

Tracks performance regressions.

**Features:**
- Benchmark execution on every push
- Comparison against main branch for PRs
- Performance alerts for >20% regressions
- Historical performance tracking

## Local Development

### Pre-commit Validation

Run the local CI script before pushing:

```bash
./scripts/local-ci.sh
```

This script runs:
- Compilation check
- Code formatting
- Clippy linting
- All tests (multiple feature combinations)
- Documentation tests
- Examples
- Security audit (if tools installed)

### Required Tools

Install additional tools for full local validation:

```bash
# Security auditing
cargo install cargo-audit

# Performance benchmarks
cargo install cargo-criterion

# Documentation tools
cargo install cargo-doc
```

## Configuration Files

### Code Quality
- **`rustfmt.toml`**: Code formatting rules
- **`clippy.toml`**: Linting configuration
- **`deny.toml`**: Dependency and license checking

### Automation
- **`.github/dependabot.yml`**: Automated dependency updates
- **`.github/workflows/`**: All CI/CD workflows

## Release Process

### Automated Release

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md` with new version
3. Commit changes: `git commit -m "Release v1.0.0"`
4. Create and push tag: `git tag v1.0.0 && git push origin v1.0.0`
5. GitHub Actions automatically handles the rest

### Manual Release (if needed)

```bash
# Build release binary
cargo build --release

# Run final tests
cargo test --all-features

# Publish to crates.io
cargo publish
```

## Monitoring and Alerts

### Performance Alerts
- Benchmarks that regress >20% trigger alerts
- Comments posted on PRs with performance changes
- Historical tracking prevents performance drift

### Security Alerts
- Daily security scans catch new vulnerabilities
- Dependency updates via Dependabot
- License compliance checking

### Quality Gates
- All tests must pass before merge
- Code coverage tracking
- Documentation must build successfully
- Examples must run without errors

## Troubleshooting

### Common CI Failures

**Formatting Issues:**
```bash
cargo fmt --all
```

**Clippy Warnings:**
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

**Test Failures:**
```bash
cargo test --all-features
```

**Documentation Issues:**
```bash
cargo doc --all-features --no-deps
```

### Platform-Specific Issues

**Windows:**
- Line ending differences (use `git config core.autocrlf true`)
- Path separator issues in tests

**macOS:**
- ARM64 vs x86_64 differences
- Different system dependencies

**Linux:**
- musl vs glibc targets
- Static linking considerations

## Performance Optimization

### CI Speed
- Dependency caching with `Swatinem/rust-cache`
- Parallel job execution
- Incremental compilation
- Targeted testing based on changes

### Resource Usage
- Limited parallel jobs to prevent resource exhaustion
- Timeout handling for long-running tests
- Efficient Docker layer caching

## Security Considerations

### Secrets Management
- `CARGO_REGISTRY_TOKEN` for crates.io publishing
- `GITHUB_TOKEN` for release automation
- No secrets in logs or outputs

### Supply Chain Security
- Pinned action versions
- Dependency vulnerability scanning
- License compliance checking
- Code signing for releases

## Metrics and Analytics

### Code Coverage
- Generated by `cargo-tarpaulin`
- Uploaded to Codecov
- Tracked over time

### Performance Metrics
- Parsing speed benchmarks
- Memory usage tracking
- Regression detection

### Quality Metrics
- Test success rates
- CI pipeline reliability
- Release frequency

## Future Improvements

- [ ] Fuzzing integration
- [ ] Property-based testing
- [ ] Cross-compilation testing
- [ ] Integration with external services
- [ ] Mobile platform support
- [ ] WASM target testing