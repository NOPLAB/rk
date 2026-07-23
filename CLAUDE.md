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

# Run a single test by name
cargo test -p rk-core test_name

# Run tests with output
cargo test -- --nocapture

# Check without building
cargo check

# Format code
cargo fmt

# Lint
cargo clippy

# Build with CAD kernel (Truck is default)
cargo build                              # Uses Truck (default, Pure Rust B-Rep)
cargo build --features rk-cad/opencascade  # Use OpenCASCADE instead (requires fixing)
cargo build --no-default-features        # No CAD kernel (NullKernel)
```

## Architecture

RK is a 3D CAD editor built with Rust, evolving into an agentic platform
where AI agents drive CAD (and later simulation) through a headless
engine. The codebase is a Cargo workspace with five crates:

### Crate Dependencies

```
rk-frontend (egui application; UI state + rendering glue only)
    ├── rk-engine (headless engine: document, commands, events, undo)
    │       ├── rk-core (data structures)
    │       └── rk-cad (CAD kernel abstraction)
    └── rk-renderer (wgpu rendering)
            └── rk-core
```

### rk-core

Core data structures and logic:

- `Part`: Mesh with metadata and joint points
- `Assembly`: Scene graph for hierarchical structure
- `Project`: Serializable project file (RON format, `.rk` extension)
- Import formats: STL, OBJ, DAE (Collada), URDF
- Export formats: URDF

### rk-cad

CAD kernel abstraction and parametric modeling:

- **Kernel abstraction** (`CadKernel` trait): Interface for geometry backends (OpenCASCADE, Truck, or NullKernel)
- **Sketch system**: 2D sketches with entities (points, lines, arcs, circles) and constraints (coincident, parallel, perpendicular, dimensions)
- **Constraint solver**: Newton-Raphson iteration for sketch constraint solving
- **Feature operations**: Extrude, revolve, boolean operations on sketches to create 3D solids
- **Parametric history**: Ordered feature list with rollback/rebuild support

### rk-engine

Headless CAD engine — the single owner and mutator of all domain state.
GUI and (future) agent frontends are both clients:

- `Document`: `rk_core::Project` + `rk_cad::CadData`, persisted as RON v2
  (`.rk`). v1 files load with empty CAD data; bodies are rebuilt on load
- `Command` / `Event`: serde enums (JSON-tagged) — the only mutation path
  and the change-notification stream. Events carry IDs; bulk data
  (meshes) is pulled from the engine by ID
- `Engine::apply(cmd) -> Result<Vec<Event>>`: atomic (snapshot rollback
  on failure). `apply_interactive`/`end_interaction` coalesce a drag
  into one undo step, with cancel support
- Undo/redo: full-document snapshots (max 50) + an append-only command
  journal (`command_log()`) for auditing and future branching
- `preview_extrude`: pure query for dialog previews
- Adding a Command/Event variant: update the exhaustive lists in
  `crates/rk-engine/tests/serde_roundtrip.rs` (compile error reminds you)

### rk-renderer

WGPU-based 3D renderer with plugin architecture:

- `SubRenderer` trait: Interface for custom renderers
- `RendererRegistry`: Plugin system for sub-renderers
- `RenderContext`: GPU context abstraction
- `Scene` / `RenderObject`: Scene management
- `MeshManager`: GPU mesh resource management
- Built-in sub-renderers: Grid, Mesh, Axis, Marker, Gizmo, Collision, Sketch, PlaneSelector
- Render priorities in `sub_renderers::priorities`: GRID(0) → SKETCH(50) → MESH(100) → AXIS(200) → MARKER(300) → COLLISION(350) → PLANE_SELECTOR(400) → GIZMO(1000)

### rk-frontend

egui-based GUI application. Owns UI state only — all domain state lives
in the engine:

- `AppState`: selection, `EditorMode` (Assembly/PlaneSelection/Sketch),
  tools, dialogs, and a `SharedEngine` handle. `SharedAppState` =
  `Arc<Mutex<AppState>>`
- `AppAction`: UI-only variants (`SelectPart`, `SketchUi(...)`) plus
  `Cmd(Command)`, `Interactive`/`EndInteraction` (drag sessions), and
  `Composite(...)` (UI + command combos)
- `actions/`: dispatch (`mod.rs`), UI handlers (`ui.rs`), composites
  (`composite.rs`), constraint workflow (`constraints.rs`)
- `sync.rs`: `apply_events` — the single place engine events become
  renderer updates; `DocumentReset` triggers a full scene rebuild
- `SketchModeState`: tools, selection, and coordinate-based
  `InProgressEntity` previews (shapes commit as one atomic command)
- Panels in `panels/` module for UI components

## Key Patterns

- **Command/Event**: panels queue `AppAction`s; per frame the dispatcher
  applies engine commands and `sync::apply_events` updates the renderer.
  Never mutate domain state directly — add a `Command` instead
- **Interaction sessions**: continuous edits (gizmo drags, DragValue
  bursts) use `apply_interactive` under one session ID so the whole
  gesture is a single undo step
- **Engine reads**: lock briefly, clone what you need, release. Never
  hold the engine guard while locking the viewport in panel code
- **Plugin Renderer**: New rendering features implement `SubRenderer` trait and register with `RendererRegistry`
- **Editor Modes**: `EditorMode::Assembly` for 3D editing, `EditorMode::Sketch` for 2D sketch editing
- **CAD Kernel Abstraction**: `CadKernel` trait allows switching between geometry backends via feature flags. The engine owns the kernel (`Engine::kernel()`)

## Platform Support

- Native: Linux (X11/Wayland), Windows, macOS (WASM support was removed)

## Roadmap (Phase 0 done)

- Phase 0: headless `rk-engine` extraction — done
- Phase 1: MCP server (`rk-mcp`) + headless rendering (screenshots for agents)
- Phase 2: Tauri + React frontend, egui retirement
- Phase 3: solver integrations (rigid-body dynamics -> FEM -> CFD)
