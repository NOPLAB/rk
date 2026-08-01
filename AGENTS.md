# AGENTS.md

This file contains guidelines for agentic coding agents working in the RK repository.

## Build Commands

```bash
# Build all crates
cargo build

# Build release
cargo build --release

# Check without building
cargo check

# Format code
cargo fmt

# Lint
cargo clippy

# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p rk-core
cargo test -p rk-cad
cargo test -p rk-renderer
cargo test -p rk-engine

# Run a single test
cargo test test_name
cargo test test_name --exact
cargo test module_name::test_name

# Choosing a CAD kernel
cargo build                              # OpenCASCADE, OCCT from source (default)
cargo build -p rk-cad --no-default-features --features truck   # Truck, pure Rust
cargo build -p rk-cad --no-default-features                    # No kernel (NullKernel)
```

The default build compiles OCCT and needs CMake plus a C++ toolchain; the
first one takes tens of minutes. The kernel must be selected on `rk-cad`
itself — every other crate takes `rk-cad` with default features, so
`--no-default-features` at the workspace level unifies the default back on.
The pre-commit hook runs `cargo clippy --all-targets -- -D warnings`, which
means the same OCCT build.

After making changes, ALWAYS run `cargo clippy` and `cargo test` before submitting.

## Code Style Guidelines

### Formatting

- Use `cargo fmt` for formatting
- Follow Rust 2024 edition standards
- 100 character line limit (standard rustfmt default)
- Use spaces, not tabs

### Import Organization

Imports should follow this order:

1. Standard library (`std::*`)
2. External crates (alphabetical)
3. Internal crates (`rk_*`)
4. Local modules (`crate::*`)

Example:

```rust
use std::collections::HashMap;
use std::path::Path;

use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use rk_core::Part;
use crate::inertia::InertiaMatrix;
```

Use brace expansion for multiple imports from same crate: `use glam::{Mat4, Vec3, Quat};`

### Naming Conventions

- **Types**: `PascalCase` (structs, enums, traits)
- **Functions/Methods**: `snake_case`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Module names**: `snake_case`
- **Type aliases**: `PascalCase` (e.g., `CadResult<T>`)
- **Acronyms**: Treat as words (e.g., `Uuid`, not `UUID`)

### Documentation

- Use module-level doc comments (`//!`) to describe module purpose
- Use doc comments (`///`) for public items
- Document public fields in structs
- Include `# Example` sections for complex operations

Example:

```rust
//! CAD Kernel Abstraction
//!
//! Provides traits for geometry backends.

/// A part loaded from an STL file with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Part {
    /// Original STL file path (for re-export)
    pub stl_path: Option<String>,
}
```

### Types

- Use `Uuid` for entity IDs throughout
- Use `f32` for floating-point (GPU-friendly), not `f64`
- Use array types (`[f32; 3]`, `[f32; 4]`) for fixed-size vectors when needed for serialization
- Use glam types (`Vec3`, `Vec2`, `Mat4`, `Quat`) for math operations
- Define Result type aliases per module: `pub type CadResult<T> = Result<T, CadError>;`

### Error Handling

- Use `thiserror` for custom error types with `#[derive(Error)]`
- Return `Result<T, Error>` for fallible operations
- Use `?` operator for error propagation
- Avoid `unwrap()` and `expect()` except in tests
- Provide context with `.map_err()` or `anyhow::Context` if needed

Example:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CadError {
    #[error("Invalid geometry: {0}")]
    InvalidGeometry(String),
    #[error("Operation not supported by this kernel")]
    UnsupportedOperation,
}

pub type CadResult<T> = Result<T, CadError>;
```

### Derive Macros

Common derive combinations:

- `Debug, Clone` for all public structs
- `Serialize, Deserialize` for serializable types
- `Copy` for small, simple types (IDs, enums)
- `Default` for types with sensible defaults
- `PartialEq, Eq` for types needing comparison

Example:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId {
    pub solid_id: Uuid,
    pub index: u32,
}
```

### Pattern Matching

- Use `_` for unused match arms
- Prefer `matches!()` macro for simple checks
- Destructure enums explicitly

Example:

```rust
if matches!(self, SketchEntity::Point { .. }) {
    // handle point
}
```

### Testing

- Place tests in modules: `#[cfg(test)] mod tests { ... }`
- Use descriptive test names: `test_entity_id()`, `test_bounding_box_center()`
- Test public API only
- Use `use super::*;` in test modules

### Concurrency

- Use `parking_lot::Mutex` for locks (preferred over std)
- Use `Arc<Mutex<T>>` (`SharedEngine`) for shared state
- Keep critical sections short

### Serialization

- Use `serde` with `Serialize, Deserialize` derives
- Use `ron` format for project files (human-readable)
- Use version fields for backward compatibility

### Workspace Structure

- `rk-core`: Core data structures (Part, Assembly, Project)
- `rk-cad`: CAD kernel abstraction and parametric modeling
- `rk-engine`: Headless engine — document, commands, events, undo
- `rk-renderer`: WGPU-based 3D rendering with plugin architecture
- `rk-mcp`: MCP server that lets agents drive the engine
- `apps/desktop`: The Tauri 2 + React + Three.js application

### Architecture Patterns

- **Command/Event**: the UI sends `Command`s to the engine and syncs from the events it gets back
- **Plugin Renderer**: Implement `SubRenderer` trait and register with `RendererRegistry`
- **Shared State**: Pass `SharedEngine` (`Arc<Mutex<Engine>>`) to components
- **Kernel Abstraction**: Implement `CadKernel` trait for geometry backends

When adding new features:

1. Add data structures in appropriate crate (core/cad/renderer)
2. Implement traits/interfaces
3. Add UI in `apps/desktop` (React)
4. Write tests
5. Run clippy and fix warnings
6. Format with `cargo fmt`
