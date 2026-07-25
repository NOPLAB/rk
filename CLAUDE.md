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

# Run the MCP server (stdio; logs go to stderr)
cargo run -p rk-mcp

# Run the Tauri desktop app (dev: starts Vite + opens the window)
cd apps/desktop && npm install && npm run tauri dev

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
engine. The codebase is a Cargo workspace with six library/binary crates
under `crates/` plus the Tauri desktop app under `apps/desktop`:

### Crate Dependencies

```
rk-frontend (egui application; UI state + rendering glue only)
    ├── rk-engine (headless engine: document, commands, events, undo)
    │       ├── rk-core (data structures)
    │       └── rk-cad (CAD kernel abstraction)
    └── rk-renderer (wgpu rendering)
            └── rk-core

rk-mcp (MCP server for agents; stdio transport)
    ├── rk-engine
    └── rk-renderer (headless: offscreen texture + readback)

rk-desktop (apps/desktop/src-tauri: Tauri 2 shell; webview renders with Three.js)
    └── rk-engine
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

### rk-mcp

MCP server (rmcp, stdio transport) that lets AI agents drive the engine.
stdout carries JSON-RPC — log to stderr only:

- Tools: `apply` (batch of `Command`s as JSON; validated first, applied
  in order), `describe_scene` (document snapshot as JSON),
  `screenshot` (headless render, PNG), `command_reference` (docs)
- `headless.rs`: surfaceless wgpu device; rebuilds a fresh
  `rk_renderer::Renderer` scene from the engine per shot, renders into a
  COPY_SRC texture, reads back and encodes PNG. GPU is initialized
  lazily on first screenshot
- `src/commands_reference.md` documents every command;
  `tests/reference_examples.rs` deserializes each ```json example and
  fails compilation when a `Command` variant is added but undocumented

### rk-desktop (apps/desktop)

Tauri 2 + React/TypeScript + Three.js desktop app (Phase 2; will replace
the egui frontend once at feature parity):

- `src-tauri/` (`rk-desktop` crate): owns a `SharedEngine`; IPC commands
  mirror the MCP protocol shape — `engine_apply` (batch of `Command`s,
  validate-all-then-apply), `engine_apply_interactive` /
  `engine_end_interaction` (drag sessions), `scene_snapshot` (part
  metadata + render transforms + body IDs + links/joints + undo state),
  `get_part_mesh` / `get_body_mesh` (bulk data pulled by ID)
- `src/` (React): `engine/api.ts` typed IPC wrappers, `engine/commands.ts`
  command builders, `engine/interaction.ts` drag-session helpers
  (latest-wins coalescing), `scene/viewport.ts` Three.js scene manager
  (Z-up; event-driven sync mirroring `rk-frontend/src/sync.rs`;
  TransformControls gizmo), `components/` panels
- Gizmo drags run through `apply_interactive` under one session ID, so a
  whole drag is one undo step; Escape cancels (engine rolls back).
  Parts store their origin in the owning link's frame, so the world
  matrix from the gizmo is converted with `parent_transform⁻¹`
- Dev: `npm run tauri dev` (Vite on port 1420); the production build embeds
  `dist/` via the `custom-protocol` feature

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

## Roadmap (Phases 0-1 done, Phase 2 in progress)

- Phase 0: headless `rk-engine` extraction — done
- Phase 1: MCP server (`rk-mcp`) + headless rendering — done
- Phase 2: Tauri + React frontend (`apps/desktop`) — scaffold + viewer +
  part editing + joint UI + mesh/URDF import-export done; sketch/feature
  UI, gizmo (`apply_interactive`) and egui retirement pending
- Phase 3: solver integrations (rigid-body dynamics -> FEM -> CFD)
