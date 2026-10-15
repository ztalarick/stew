# Stew

A statically typed, interpreted programming language and toolchain written in Rust.

## Workspace Structure

The project is structured as a Cargo workspace with the following crates:

* [`crates/stew-core`](crates/stew-core): Core AST definitions, type checking, semantic analysis, and the interpreter runtime.
* [`crates/stew-parser`](crates/stew-parser): Translates Tree-sitter Concrete Syntax Trees (CST) into the Core Abstract Syntax Tree (AST).
* [`crates/stew-cli`](crates/stew-cli): Command-line interface and diagnostic error reporting.
* [`crates/stew-lsp`](crates/stew-lsp): Language Server Protocol implementation.
* [`tree-sitter-stew`](tree-sitter-stew): Tree-sitter grammar specification and generated C/Rust bindings.

---

## Building and Compiling

### Build All Crates

```bash
cargo build
```

For release/optimized binaries:

```bash
cargo build --release
```

### Fast Type & Syntax Check

```bash
cargo check --workspace
```

### Build Specific Modules

Build individual packages using the `-p` (or `--package`) flag:

```bash
# Build the CLI binary
cargo build -p stew-cli

# Build stew-core library
cargo build -p stew-core

# Build stew-parser library
cargo build -p stew-parser

# Build stew-lsp server
cargo build -p stew-lsp

# Build tree-sitter bindings
cargo build -p tree-sitter-stew
```

### Running the CLI

Run the CLI directly with Cargo:

```bash
cargo run -p stew-cli -- <args>
```

---

## Running Tests

### Run All Workspace Tests

```bash
cargo test --workspace
```

To see standard output and tracing logs during test execution:

```bash
cargo test --workspace -- --nocapture
```

### Run Tests for Specific Modules

```bash
# Core logic and AST tests
cargo test -p stew-core

# Parser tests
cargo test -p stew-parser

# CLI tests
cargo test -p stew-cli

# LSP tests
cargo test -p stew-lsp

# Tree-sitter Rust binding tests
cargo test -p tree-sitter-stew
```

### Run a Single Test / Filter Tests

Filter tests by name across the workspace or within a specific package:

```bash
cargo test -p <package-name> <test_name_filter>
```

---

## Tree-sitter Grammar Development

When working on grammar rules inside [`tree-sitter-stew`](tree-sitter-stew) (`grammar.js`):

```bash
cd tree-sitter-stew

# Regenerate parser C source from grammar.js
npx tree-sitter generate

# Run corpus parse tests
npx tree-sitter test

# Format grammar definition
npm run format
```
