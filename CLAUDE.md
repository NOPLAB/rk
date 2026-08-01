# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Build all crates
cargo build

# Build release
cargo build --release

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

# Choosing a CAD kernel
cargo build                              # OpenCASCADE, OCCT compiled from source (default)
cargo build -p rk-cad --no-default-features --features truck   # Truck, pure Rust
cargo build -p rk-cad --no-default-features                    # No kernel (NullKernel)
```

The default build compiles OCCT out of `occt-sys`, so it needs CMake and a
C++ toolchain, and the first build takes tens of minutes and about 1.5 GB
under `target/`. Two things make it work at all: `.cargo/config.toml` sets
`CMAKE_POLICY_VERSION_MINIMUM`, because OCCT's own CMakeLists asks for a
minimum CMake 4 refuses outright; and `crates/rk-cad/build.rs` links
`advapi32` on Windows, which OCCT's OSD layer needs and `opencascade-sys`
never asks for.

The kernel choice has to be made on `rk-cad` itself (`-p rk-cad`): every
other crate depends on `rk-cad` with default features, so a plain
`cargo build --no-default-features` at the workspace level unifies the
default straight back on.

## Architecture

RK is a 3D CAD editor built with Rust, evolving into an agentic platform
where AI agents drive CAD (and later simulation) through a headless
engine. The codebase is a Cargo workspace with five library/binary crates
under `crates/`, plus two applications under `apps/`: the Tauri desktop
app (`apps/desktop`) and the documentation site (`apps/docs`):

### Crate Dependencies

```
rk-desktop (apps/desktop/src-tauri: Tauri 2 shell; webview renders with Three.js)
    └── rk-engine (headless engine: document, commands, events, undo)
            ├── rk-core (data structures)
            └── rk-cad (CAD kernel abstraction)

rk-mcp (MCP server for agents; stdio transport)
    ├── rk-engine
    └── rk-renderer (wgpu rendering, headless: offscreen texture + readback)
            └── rk-core
```

`rk-renderer` is now reached only by `rk-mcp` — the desktop app draws with
Three.js in the webview. It stays because an agent still needs eyes.

### rk-core

Core data structures and logic:

- `Part`: Mesh with metadata and joint points
- `Assembly`: Scene graph for hierarchical structure
- `Project`: Serializable project file (RON format, `.rk` extension)
- Import formats: STL, OBJ, DAE (Collada), URDF, and STEP through the kernel
  (the `cad` feature, which `rk-engine` turns on). STEP files declare their
  own units and OpenCASCADE normalises them to millimetres, so `step.rs`
  scales by 0.001 into the metre-based scene — the caller's `StlUnit` is for
  formats that declare nothing. `import_mesh` goes through `load_mesh_multi`
  because a STEP assembly is one part per solid
- Export formats: URDF

### rk-cad

CAD kernel abstraction and parametric modeling:

- **Kernel abstraction** (`CadKernel` trait): interface for geometry
  backends. OpenCASCADE is the default; truck and `NullKernel` are the
  alternatives, and `default_kernel()` prefers OpenCASCADE whenever its
  feature is on. The two real backends are not equals — truck cannot
  subtract at all, so `BooleanOp::Cut` and any island in a revolve exist
  only under OpenCASCADE. Nothing outside `kernel/opencascade.rs` may
  assume which one is running; a message that names a kernel gets it from
  `kernel.name()`
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
  which regions to build, empty meaning all of them. The kernel's
  `extrude_region`/`revolve_region` take the holes with them, by whichever
  route their backend has: truck attaches a face from outer + hole wires in
  one go, since it cannot subtract; OpenCASCADE's bindings expose no way to
  give a face its islands, so it sweeps each island too and cuts that back
  out. A washer comes out of both
- **Parametric history**: Ordered feature list with rollback/rebuild support.
  `FeatureGroup` bundles timeline entries under one name for the browser and
  nothing else — the build order, the bodies and the rollback position are
  untouched, so grouping can never change the model. A feature belongs to at
  most one group (adding it to a second moves it) and a group that loses its
  last member is deleted with it

**Working on `kernel/opencascade.rs`** — five traps, all of them silent:

- A getter on a maker that has not succeeded **throws**, and a C++ exception
  crossing the cxx bridge aborts the process rather than becoming an `Err`.
  Check `IsDone()` before `Shape()`/`Face()`/`Edge()`, every time
- …but `IsDone()` before anything has built is false for a perfectly good
  shape. The sweeps (`MakePrism`, `MakeRevol`) and the booleans build in
  their constructor; the primitives (`MakeBox`, `MakeCylinder`, `MakeSphere`)
  and the modifiers (fillet, chamfer, shell, loft) need an explicit `Build`
  first. `tests/primitives.rs` exists because guarding the first group like
  the second made every primitive return an error
- **`angle` is an `f32` and cannot hold 2π.** The nearest float, and what
  anything asking for a full revolution sends, is 1.7e-7 rad _past_ a whole
  turn — and OpenCASCADE dutifully sweeps a solid that laps itself, which
  meshes to nothing at all. `full_turns_stay_full` snaps it back. A
  revolution of _no_ angle throws out of the constructor, so it is rejected
  before the call
- `TopExp_Explorer` yields each edge once per face it borders — a box comes
  back as 24 edges. `edge_key` collapses them by position, and `get_edges`,
  `fillet` and `chamfer` must number them the same way or a fillet lands on
  the wrong edge
- The deflection `BRepMesh` wants is an absolute length, not a fraction, so
  the app's one constant would be coarser than most parts are big.
  `mesh_of` bounds it by the body's own diagonal, and takes OpenCASCADE's
  surface normals rather than re-deriving them from the triangle soup
- `tessellate` holds the kernel's lock across the meshing: copying a
  `TopoDS_Shape` shares the topology underneath, and meshing writes the
  triangulation into it

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

Tauri 2 + React/TypeScript + Three.js desktop app — the application. It
replaced the egui frontend, which was deleted once it reached parity:

- `src-tauri/` (`rk-desktop` crate): owns a `SharedEngine`; IPC commands
  mirror the MCP protocol shape — `engine_apply` (batch of `Command`s,
  validate-all-then-apply), `engine_apply_interactive` /
  `engine_end_interaction` (drag sessions), `scene_snapshot` (part
  metadata + physics + render transforms + body IDs + links/joints +
  collisions + sketches + feature history + feature groups + undo state),
  `sketch_geometry` (entities with point references resolved to 2D
  coordinates, plus the constraint list), `get_part_mesh` /
  `get_body_mesh` (bulk data pulled by ID), and `open_panel_window` /
  `close_panel_window` / `floating_panels` for the torn-off panels
- `src/` (React): `engine/api.ts` typed IPC wrappers, `engine/commands.ts`
  command builders, `engine/constraints.ts` the sketch-constraint catalog,
  `engine/interaction.ts` drag-session helpers (latest-wins coalescing),
  `applyAtomic` and `newUuid`, `scene/viewport.ts` Three.js
  scene manager (Z-up; event-driven sync — engine events, never a poll;
  TransformControls gizmo),
  `scene/sketchLayer.ts` + `scene/sketchTools.ts` sketch mode,
  `scene/viewCube.ts` viewport overlays, `ui/` shared state,
  `components/` chrome
- **UI shell (modelled on Autodesk Inventor)**: a five-row grid — quick-access
  title bar, ribbon, workspace (three docks), document
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
- **Panels are tabs** (`ui/layout.ts`): the model browser, the 3D view and
  the inspector each live in one of three docks, or floating in their own OS
  window, or hidden — that one structure is what the tab strip, the drag
  between docks and the tear-off all read and write. The dock holding the 3D
  view is the one that stretches. The layout is kept in localStorage, and a
  stored layout is repaired on load rather than trusted, since a bad one
  would come up as a blank window. Dragging uses pointer capture rather than
  HTML5 drag-and-drop (`ui/panelDrag.ts`): with the pointer captured the move
  events keep coming after it leaves the window, and `screenX`/`screenY` say
  where on the desktop it was let go — which is what makes "drag the tab onto
  the second monitor" possible at all. The ghost and the drop highlight are
  plain DOM, because a pointermove at 120 Hz must not re-render the tree
- A torn-off panel is a second Tauri window running the same app; it reads
  its own window label (`panel-<id>`) to know which single panel to draw.
  The two share nothing but the engine, so `engine_apply` broadcasts
  `rk://document-changed` and the other window re-pulls. `open_panel_window`
  is `async` **on purpose**: a synchronous Tauri command runs on the main
  thread, and building a webview from there yields a window whose webview
  never attaches — an empty white frame. Closing the window emits
  `rk://panel-closed` from the Rust destroy handler, which is what docks the
  panel back; a webview cannot be relied on to report its own death
- Moving the 3D view to another dock remounts its canvas and therefore builds
  a new `Viewport`; `cameraState()`/`restoreCamera()` carry the view across so
  the model does not appear to jump. Switching tabs inside a dock only hides
  the panel (`display: none`) — unmounting would throw away the WebGL context
- Right-click menus all come from `ui/menus.ts` and render through
  `components/ContextMenu.tsx`, so a part in the browser tree and the same
  part in the 3D view offer the same commands. `Viewport.onContextMenu`
  resolves what the pointer is over (part / sketch region / empty space, or
  the sketch being edited) before the menu is built, so no caller needs the
  camera. Rename items open `components/TextPromptDialog.tsx` rather than
  `window.prompt`, which the webview styles nothing like the rest of the app
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
- The ribbon shows one button per shape, not one per variant: the same file
  groups the tools into families (`CREATE_FAMILIES` / `MODIFY_FAMILIES`) and
  `RibSplit` gives each a caret with the rest, the way Fusion nests 2-point /
  3-point / centre rectangles under "Rectangle". The button's face is the
  variant last chosen (`AppApi.toolVariant`, recorded by `setSketchTool`), so
  it always says what pressing it will do. The flyout is `position: fixed`
  because the ribbon clips its own overflow
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

- **Command/Event**: the UI sends `Command`s through `AppApi.run` — which
  is `engine_apply` — and the events that come back drive the Three.js
  scene. Never mutate domain state directly; add a `Command` instead
- **Interaction sessions**: continuous edits (gizmo drags, dimension
  spinners) use `apply_interactive` under one session ID so the whole
  gesture is a single undo step
- **Engine reads**: lock briefly, clone what you need, release. Never hold
  the engine guard across another lock or across an IPC reply
- **Plugin Renderer**: New rendering features implement `SubRenderer` trait and register with `RendererRegistry`
- **Editor modes**: the desktop app is in sketch mode when a sketch is being
  edited (`SketchLayer` owns the plane and the tools) and in assembly mode
  otherwise — the ribbon tab follows that state rather than setting it
- **CAD Kernel Abstraction**: `CadKernel` trait allows switching between geometry backends via feature flags. The engine owns the kernel (`Engine::kernel()`)

## Platform Support

- Native: Linux (X11/Wayland), Windows, macOS (WASM support was removed)

## Roadmap (Phases 0-2 done, Phase 3 next)

- Phase 0: headless `rk-engine` extraction — done
- Phase 1: MCP server (`rk-mcp`) + headless rendering — done
- Phase 2: Tauri + React frontend (`apps/desktop`) — done. Scaffold, viewer,
  part editing, joint UI, mesh/URDF import-export, the gizmo on
  `apply_interactive`, sketch/feature UI, collisions and mass/inertia,
  sketch constraints and dimensions, the Inventor-style shell (ribbon,
  model browser, ViewCube, navigation bar), the Fusion sketching flow
  (pick a plane or a face in the 3D view, the full tool set, sketches that
  stay visible, click a region to extrude it), dockable panel tabs that tear
  off into their own window, feature groups, context menus throughout — and
  finally the egui frontend deleted, with OpenCASCADE promoted to the
  default kernel so `Cut` and STEP import work in an ordinary build
- Phase 3: solver integrations (rigid-body dynamics -> FEM -> CFD)
