# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Build all crates
cargo build

# Build release
cargo build --release

# Run the application
cargo run -p rk-frontend

# Run tests
cargo test

# Run tests for a specific crate
cargo test -p rk-core

# Check without building
cargo check

# Format code
cargo fmt

# Lint
cargo clippy

# Build with CAD kernel (optional features)
cargo build --features rk-cad/truck    # Pure Rust B-Rep kernel
cargo build --features rk-cad/opencascade  # OpenCASCADE bindings
```

## Architecture

RK is a 3D CAD editor built with Rust. The codebase is organized as a Cargo workspace with four crates:

### Crate Dependencies

```
rk-frontend (egui application)
    ├── rk-cad (CAD kernel abstraction)
    └── rk-renderer (wgpu rendering)
            └── rk-core (data structures)
```

### rk-core

Core data structures and logic:

- `Part`: Mesh with metadata and joint points
- `Assembly`: Scene graph for hierarchical structure
- `Project`: Serializable project file (RON format)
- Mesh import/export (STL, OBJ, DAE, URDF)

### rk-cad

CAD kernel abstraction and parametric modeling:

- **Kernel abstraction** (`CadKernel` trait): Interface for geometry backends (OpenCASCADE, Truck, or NullKernel)
- **Sketch system**: 2D sketches with entities (points, lines, arcs, circles) and constraints (coincident, parallel, perpendicular, dimensions)
- **Constraint solver**: Newton-Raphson iteration for sketch constraint solving
- **Feature operations**: Extrude, revolve, boolean operations on sketches to create 3D solids
- **Parametric history**: Ordered feature list with rollback/rebuild support

### rk-renderer

WGPU-based 3D renderer with plugin architecture:

- `SubRenderer` trait: Interface for custom renderers
- `RendererRegistry`: Plugin system for sub-renderers
- `RenderContext`: GPU context abstraction
- `Scene` / `RenderObject`: Scene management
- `MeshManager`: GPU mesh resource management
- Built-in sub-renderers: Grid, Mesh, Axis, Marker, Gizmo, Collision, Sketch
- Render priorities defined in `sub_renderers::priorities`

### rk-frontend

egui-based GUI application:

- `AppState`: Central application state with action queue pattern
- `AppAction`: Enum defining all possible state mutations (file, part, assembly, joint, collision, sketch actions)
- `SharedAppState`: Thread-safe state wrapper (`Arc<Mutex<AppState>>`)
- `CadState`: CAD-specific state including `EditorMode` (Assembly/Sketch modes)
- `SketchModeState`: Sketch editing state with tools, selection, and in-progress entities
- Panels in `panels/` module for UI components

## Key Patterns

- **Action Queue**: UI components queue `AppAction` variants, which are processed centrally in the update loop
- **Plugin Renderer**: New rendering features implement `SubRenderer` trait and register with `RendererRegistry`
- **Shared State**: `SharedAppState` (`Arc<Mutex<AppState>>`) is passed to panels and the renderer
- **Editor Modes**: `EditorMode::Assembly` for 3D editing, `EditorMode::Sketch` for 2D sketch editing
- **CAD Kernel Abstraction**: `CadKernel` trait allows switching between geometry backends via feature flags

## Platform Support

- Native: Linux (X11/Wayland), Windows, macOS
- WASM: Web browser support with conditional compilation (`cfg(target_arch = "wasm32")`)
