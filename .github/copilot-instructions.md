# Copilot Instructions for pg_lexo

## Project Overview

`pg_lexo` is a PostgreSQL extension written in Rust using [pgrx](https://github.com/pgcentralfoundation/pgrx) for generating lexicographic ordering values. It enables efficient reordering of items in database tables without requiring updates to other rows.

### Key Features
- **Base62 Encoding**: Uses 62 characters (0-9, A-Z, a-z) for compact position strings
- **Lexicographic Ordering**: Positions sort correctly using standard string comparison
- **Efficient Insertions**: Insert items between any two positions without updating other rows
- **PostgreSQL Compatibility**: Supports PostgreSQL 16, 17, and 18

## Development Environment

### Prerequisites
- Rust (latest stable) - Install via [rustup.rs](https://rustup.rs/)
- PostgreSQL 16-18 with development headers
- [cargo-pgrx](https://github.com/pgcentralfoundation/pgrx) version 0.16.1

### Setup Commands
```bash
# Install cargo-pgrx
cargo install cargo-pgrx --version "0.16.1" --locked

# Initialize pgrx (downloads and configures PostgreSQL)
cargo pgrx init
```

## Building and Testing

### Build Commands
```bash
# Build the extension
cargo build

# Build for a specific PostgreSQL version
cargo pgrx package --features pg17 --pg-config $(which pg_config)
```

### Test Commands
```bash
# Run unit tests (no PostgreSQL required)
cargo test

# Run PostgreSQL integration tests
cargo pgrx test pg17  # Replace with your PG version (pg16, pg17, pg18)
```

### Linting
```bash
# Run Clippy for linting
cargo clippy

# Format code
cargo fmt
```

## Code Architecture

### Directory Structure
- `src/lib.rs` - Main extension code with all functions and tests
- `src/bin/pgrx_embed.rs` - pgrx binary embedding
- `pg_lexo.control` - PostgreSQL extension control file
- `.github/workflows/` - CI/CD workflows

### Key Components
- **lexo schema**: All public functions are under the `lexo` schema (e.g., `lexo.first()`, `lexo.between()`)
- **Base62 encoding**: Character set `0-9, A-Z, a-z` for position strings
- **Position generation functions**: `generate_after()`, `generate_before()`, `generate_between()`

### Public API Functions
| Function | Description |
|----------|-------------|
| `lexo.first()` | Returns the initial position `'V'` |
| `lexo.after(position TEXT)` | Returns a position after the given position |
| `lexo.before(position TEXT)` | Returns a position before the given position |
| `lexo.between(before TEXT, after TEXT)` | Returns a position between two positions |

## Code Style and Conventions

### Rust Conventions
- Use Rust 2024 edition features
- Follow standard Rust naming conventions (snake_case for functions/variables, CamelCase for types)
- Add documentation comments (`///`) for public functions with examples
- Use `#[pg_extern]` attribute for PostgreSQL-exposed functions
- Use `#[pg_schema]` for organizing functions into schemas

### Testing Conventions
- Unit tests go in `mod unit_tests` (no PostgreSQL required)
- Integration tests use `#[pg_test]` attribute in `mod tests`
- Test both valid inputs and edge cases (empty strings, invalid order, etc.)

### Documentation
- Include SQL examples in doc comments using triple-backtick code blocks
- Document all public API functions with `# Arguments`, `# Returns`, and `# Example` sections

## PostgreSQL Feature Flags
The extension uses feature flags for PostgreSQL version compatibility:
- `pg16` - PostgreSQL 16
- `pg17` - PostgreSQL 17 (default)
- `pg18` - PostgreSQL 18

## Contributing Guidelines

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes following the code style conventions
4. Add tests for new functionality
5. Run `cargo test` and `cargo clippy` before committing
6. Commit your changes with descriptive messages
7. Open a Pull Request

## Common Tasks

### Adding a New Function
1. Add the function in `src/lib.rs` within the `lexo` module
2. Use `#[pg_extern]` attribute to expose it to PostgreSQL
3. Add documentation with SQL examples
4. Add unit tests in `mod unit_tests`
5. Add integration tests in `mod tests` using `#[pg_test]`

### Updating pgrx Version
1. Update version in `Cargo.toml` for both `pgrx` and `pgrx-tests`
2. Update `cargo-pgrx` installation commands in:
   - `.github/workflows/build-extension.yml`
   - `Dockerfile`
   - `README.md`
3. Re-run `cargo pgrx init`
