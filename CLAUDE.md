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

# Run the documentation site (npm workspace at the repo root)
npm install && npm run docs:dev

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
under `crates/`, plus two applications under `apps/`: the Tauri desktop
app (`apps/desktop`) and the documentation site (`apps/docs`):

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
- **Sketch system**: 2D sketches with entities (points, lines, arcs,
  circles, ellipses, splines) and constraints (coincident, parallel,
  perpendicular, dimensions). A sketch's plane is a free
  `{origin, normal, x_axis, y_axis}` frame, so a sketch can sit on a
  solid's face just as well as on an origin plane
- **Region extraction** (`sketch/profile.rs`): every closed area a sketch
  encloses. Curves are flattened, split wherever they cross, and stitched
  into a planar graph whose half-edge walk yields one loop per area; a loop
  inside another becomes its hole. So a line drawn across a rectangle
  really does divide it, and a circle inside one makes a plate with a hole
  rather than two overlapping solids. `Profile::id` is derived from the
  curves that bound the region, so a feature keeps pointing at the same
  area when the sketch is edited elsewhere
- **Constraint solver**: Newton-Raphson iteration for sketch constraint
  solving. Only point coordinates are variables, so radius/diameter/equal-radius
  dimensions are applied to the circle as assignments before the iteration
  (and left out of the DOF count) instead of being solved for
- **Feature operations**: Extrude, revolve, boolean operations on sketches
  to create 3D solids. `Extrude`/`Revolve` carry `profiles: Vec<Uuid>` —
  which regions to build, empty meaning all of them. `CadKernel::
  extrude_region`/`revolve_region` take the holes with them: truck attaches
  a face from outer + hole wires in one go, so a washer never needs the
  boolean subtract that backend does not have
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
  metadata + physics + render transforms + body IDs + links/joints +
  collisions + sketches + feature history + undo state),
  `sketch_geometry` (entities with point references resolved to 2D
  coordinates, plus the constraint list), `get_part_mesh` /
  `get_body_mesh` (bulk data pulled by ID)
- `src/` (React): `engine/api.ts` typed IPC wrappers, `engine/commands.ts`
  command builders, `engine/constraints.ts` the sketch-constraint catalog,
  `engine/interaction.ts` drag-session helpers (latest-wins coalescing),
  `applyAtomic` and `newUuid`, `scene/viewport.ts` Three.js
  scene manager (Z-up; event-driven sync mirroring
  `rk-frontend/src/sync.rs`; TransformControls gizmo),
  `scene/sketchLayer.ts` + `scene/sketchTools.ts` sketch mode,
  `scene/viewCube.ts` viewport overlays, `ui/` shared state,
  `components/` chrome
- **UI shell (modelled on Autodesk Inventor)**: a five-row grid — quick-access
  title bar, ribbon, workspace (browser | viewport | inspector), document
  tabs, status bar. `components/Ribbon.tsx` owns the tab strip and the File
  drop-down, `components/ribbonTabs.tsx` holds what each tab can do (3D
  Model / Sketch / Assembly / View) and `ribbonParts.tsx` the group plus big
  and small button primitives. Editing a sketch switches to the Sketch tab
  and leaving it returns to 3D Model. `components/BrowserPanel.tsx` is the
  model tree (origin planes, parts, the link/joint chain, sketches, and the
  feature history ending in an "End of Part" marker that doubles as the
  rollback control). `components/Inspector.tsx` swaps between part
  properties and the sketch's constraint list. Icons are inline two-tone SVG
  in `components/icons.tsx` — no icon font or CDN, since the app ships as one
  self-contained window
- Every chrome component takes one `AppApi` (`ui/appApi.ts`) instead of a
  dozen props; App owns the state and all mutation still goes through `run`.
  `ui/fileActions.ts` is the single implementation of the document commands,
  shared by the quick-access bar, the File menu and the Ctrl+S/O shortcuts
- `scene/viewCube.ts`: the ViewCube and axis triad render into scissored
  corners of the main canvas (one extra draw call, no second GPU context).
  A click resolves to a direction from where it lands on the cube — an axis
  joins in once the hit is far enough along it, so faces, edges and corners
  all fall out of one test. Three builds box faces for a Y-up world, so each
  label carries a quarter-turn correction (`FACE_SPIN`)
- Sketch overlay objects all skip the depth test and sit on the same plane,
  so `SketchLayer`'s constructor assigns each an explicit `renderOrder`;
  without it the selection highlight is drawn under the plain curve and
  never shows. `Viewport` registers its canvas listeners with an
  `AbortController` so a dev reload does not leave the old ones attached
- Gizmo drags run through `apply_interactive` under one session ID, so a
  whole drag is one undo step; Escape cancels (engine rolls back).
  Parts store their origin in the owning link's frame, so the world
  matrix from the gizmo is converted with `parent_transform⁻¹`
- Sketch mode: the layer's group carries the plane basis, so geometry is
  built in 2D on z = 0 and clicks are ray-plane intersections converted
  back. `SketchDrawing` holds the in-progress shape client-side and only
  emits a command once a shape is complete — one `add_sketch_entities`
  per shape (one undo step), and cancelling leaves no orphan points. The
  line tool reuses point IDs between segments and closes onto the chain's
  first point, which is what makes the region extractor find a closed loop
- Sketch plane picking follows Fusion: "Create Sketch" arms a modal pick
  (`scene/planePicker.ts`) that draws the three origin quads and accepts
  either one of those or a flat face of a solid. The kernel does not carry
  B-Rep faces through tessellation, so `scene/facePlane.ts` recovers one by
  flood-filling coplanar triangles out from the one under the pointer —
  which works on imported meshes too, and refuses a lone facet so a
  cylinder wall cannot be sketched on. The in-plane axes are pinned to the
  world axis least parallel to the normal, or the same face would give a
  different 2D frame each session
- Tools live in three files: `sketchGeom.ts` (2D maths), `sketchTools.ts`
  (creation, a stage machine so 3-click tools fit) and `sketchEdits.ts`
  (fillet/trim/extend/offset/mirror/patterns, each resolving from one
  click plus the ribbon's numbers). `sketchToolInfo.ts` is the one table of
  labels, icons and prompts, shared by the ribbon and the status bar. An
  edit tool that declines returns a `problem` string that lands in the
  status bar, rather than looking broken
- Arcs have no direction flag: the engine stores a counter-clockwise sweep
  from `start` to `end`, so a clockwise arc is emitted with the endpoints
  swapped. Getting this backwards turns a slot's end caps inside out
- Finished sketches stay on screen (`scene/idleSketches.ts`), one child
  group per sketch carrying its own plane basis, drawn with the depth test
  on so a solid built from a sketch hides it. Their regions are filled
  meshes (`RegionFills`, three's ShapeGeometry with `shape.holes`) and
  clicking one selects it — that selection is what Extrude and Revolve
  build. Inside sketch mode the active layer owns the same job, picked by
  point-in-polygon in sketch coordinates rather than a raycast
- Trimming deletes the points it orphans, and fillet looks past the nearest
  point for one that actually joins two lines — without both, every edit
  leaves loose points that hijack the next snap
- Constraints: pick entities with the select tool (Shift adds), then hit a
  constraint — `engine/constraints.ts` is the single table of what each
  one needs, what the geometry measures now (so a dimension opens on a
  no-op) and the payload to send. Constraints are keyed by ID, so the
  dimension list edits a value by re-sending the same constraint with a
  new one. Adding and re-solving go through `applyAtomic`, which runs the
  pair in one interaction session and therefore one undo step
- Collision shapes render as wireframes from `LinkInfo.collisions`, whose
  transforms already fold in the link pose — they are rebuilt from the
  snapshot on every refresh instead of tracked through events. Collisions
  belong to links, so a part only gets them once it is in the assembly
- `tests/command_payloads.rs` applies the JSON the TypeScript builders
  emit; it is the only check that those field names match `Command`
- `.app` sets `grid-template-columns: minmax(0, 1fr)`. Without it the
  implicit `auto` track sizes to the widest row, so a ribbon tab with more
  buttons than fit stretches the whole window's chrome past the edge
  instead of scrolling inside it. The Sketch tab holds more than fits at
  any sane width, so its Exit group is absolutely positioned over the right
  edge — leaving a sketch must never depend on scrolling to find the button
- Dev: `npm run tauri dev` (Vite on port 1420); the production build embeds
  `dist/` via the `custom-protocol` feature

### rk-docs (apps/docs)

Astro + Starlight documentation site, published to GitHub Pages at
`noplab.github.io/rk` by `.github/workflows/deploy-pages.yml` on pushes to
`main`:

- The only npm workspace declared in the root `package.json`, so it
  installs and builds from the repo root (`npm run docs:dev` / `docs:build`).
  `apps/desktop` deliberately stays outside the workspace — it carries its
  own `package-lock.json` and Tauri drives its Vite build itself
- Pages live in `src/content/docs/`; `astro.config.mjs` owns the sidebar,
  where `base: '/rk'` means every hand-written internal link needs the
  `/rk/` prefix
- `public/favicon.svg` and `src/assets/logo.svg` are symlinks into the
  repo-root `assets/icons/`, so moving this directory breaks the build
  until the `../` counts are fixed

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
  part editing + joint UI + mesh/URDF import-export + gizmo
  (`apply_interactive`) + sketch/feature UI + collisions and mass/inertia
  + sketch constraints and dimensions + the Inventor-style shell (ribbon,
  model browser, ViewCube, navigation bar) + the Fusion sketching flow
  (pick a plane or a face in the 3D view, the full tool set, sketches that
  stay visible, click a region to extrude it) done; egui retirement pending
- Phase 3: solver integrations (rigid-body dynamics -> FEM -> CFD)
