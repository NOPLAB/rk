# RK Engine Command Reference

Commands are JSON objects with a `"type"` field (snake_case). Pass them to the
`apply` tool as `{"commands": [ ... ]}`. Every ```json block below is a single
valid command.

## Conventions

- **Units**: meters, kilograms, radians. The world is **Z-up**.
- **IDs** are UUID strings. Creation commands accept `"id": null` to let the
  engine mint one; the new ID is reported back in the returned events
  (`part_added`, `sketch_added`, `feature_added`, ...).
- **Poses** are `{"xyz": [x, y, z], "rpy": [roll, pitch, yaw]}` (radians).
- **Transforms** (`Mat4`) are arrays of 16 floats in **column-major** order;
  identity is `[1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]`.
- **Vectors** are arrays: `Vec3` = `[x, y, z]`, `Vec2` = `[x, y]`.
- Every command except `save_document`, `export_urdf`, `set_joint_position`,
  `rebuild_features`, `undo` and `redo` is one undo step.
- Typical modeling flow: `create_sketch` → `add_sketch_entities` (a closed
  profile) → `add_extrude` → `screenshot` to verify.

## Document / IO

Create an empty document (clears everything, not undoable):

```json
{"type": "new_document"}
```

Load / save a project file (`.rk`, RON format v2 with CAD data). `path: null`
saves to the current file:

```json
{"type": "load_document", "path": "C:/projects/robot.rk"}
```

```json
{"type": "save_document", "path": "C:/projects/robot.rk"}
```

```json
{"type": "save_document", "path": null}
```

Import a mesh file as a new part (STL/OBJ/DAE; `unit` scales STL:
`"Meters" | "Millimeters" | "Centimeters" | "Inches"`):

```json
{"type": "import_mesh", "path": "C:/meshes/arm.stl", "unit": "Millimeters"}
```

Import a URDF robot (replaces the current document) / export the assembly as
URDF (`path` is the output **directory**):

```json
{"type": "import_urdf", "path": "C:/robots/arm.urdf", "stl_unit": "Millimeters"}
```

```json
{"type": "export_urdf", "path": "C:/export", "robot_name": "my_robot"}
```

```json
{"type": "rename_project", "name": "gripper_v2"}
```

## Parts

Create a primitive part. `primitive.shape` is `"box"` (with `size`),
`"cylinder"` (with `radius`, `height`) or `"sphere"` (with `radius`):

```json
{"type": "create_primitive", "id": null, "primitive": {"shape": "box", "size": [0.2, 0.1, 0.05]}, "name": "base"}
```

```json
{"type": "create_primitive", "id": null, "primitive": {"shape": "cylinder", "radius": 0.03, "height": 0.12}, "name": "wheel"}
```

```json
{"type": "create_primitive", "id": null, "primitive": {"shape": "sphere", "radius": 0.05}, "name": null}
```

```json
{"type": "create_empty_part", "id": null, "name": "virtual_frame"}
```

```json
{"type": "delete_part", "part_id": "6b1e7c1e-8d1a-4f4e-9b1a-2f6c3d4e5a6b"}
```

```json
{"type": "rename_part", "part_id": "6b1e7c1e-8d1a-4f4e-9b1a-2f6c3d4e5a6b", "name": "base_link"}
```

Set a part's origin transform (column-major Mat4; this example translates to
x=0.1, z=0.05):

```json
{"type": "set_part_transform", "part_id": "6b1e7c1e-8d1a-4f4e-9b1a-2f6c3d4e5a6b", "transform": [1,0,0,0, 0,1,0,0, 0,0,1,0, 0.1,0,0.05,1]}
```

Appearance and physics (`color` is RGBA 0..1; inertia is a symmetric tensor):

```json
{"type": "set_part_color", "part_id": "6b1e7c1e-8d1a-4f4e-9b1a-2f6c3d4e5a6b", "color": [0.8, 0.2, 0.2, 1.0]}
```

```json
{"type": "set_part_material", "part_id": "6b1e7c1e-8d1a-4f4e-9b1a-2f6c3d4e5a6b", "material_name": "aluminum"}
```

```json
{"type": "set_part_mass", "part_id": "6b1e7c1e-8d1a-4f4e-9b1a-2f6c3d4e5a6b", "mass": 1.5}
```

```json
{"type": "set_part_inertia", "part_id": "6b1e7c1e-8d1a-4f4e-9b1a-2f6c3d4e5a6b", "inertia": {"ixx": 0.001, "ixy": 0.0, "ixz": 0.0, "iyy": 0.001, "iyz": 0.0, "izz": 0.001}}
```

Mass and inertia are also copied onto the part's link, which is what
`export_urdf` writes out.

## Assembly / Joints

Connect two parts with a fixed joint (links are created as needed; returns
`joint_added` with the new joint ID). Disconnect removes the child's joint:

```json
{"type": "connect_parts", "parent_part": "6b1e7c1e-8d1a-4f4e-9b1a-2f6c3d4e5a6b", "child_part": "9f2a4d70-3c55-4bbb-8ad9-1e0c6f7b8a9c"}
```

```json
{"type": "disconnect_part", "child_part": "9f2a4d70-3c55-4bbb-8ad9-1e0c6f7b8a9c"}
```

Joint runtime position (radians or meters; **not** an undo step — it is
kinematic state, not document data):

```json
{"type": "set_joint_position", "joint_id": "3d0f8b2a-7c4e-4d2b-9e6f-5a1b2c3d4e5f", "position": 0.7853982}
```

```json
{"type": "reset_joint_position", "joint_id": "3d0f8b2a-7c4e-4d2b-9e6f-5a1b2c3d4e5f"}
```

```json
{"type": "reset_all_joint_positions"}
```

Joint definition. `joint_type` is `"Fixed" | "Revolute" | "Continuous" |
"Prismatic" | "Floating" | "Planar"`. With `keep_child_world_pose: true` the
child part stays where it is in the world (its origin is compensated):

```json
{"type": "set_joint_type", "joint_id": "3d0f8b2a-7c4e-4d2b-9e6f-5a1b2c3d4e5f", "joint_type": "Revolute"}
```

```json
{"type": "set_joint_origin", "joint_id": "3d0f8b2a-7c4e-4d2b-9e6f-5a1b2c3d4e5f", "origin": {"xyz": [0.0, 0.0, 0.1], "rpy": [0.0, 0.0, 0.0]}, "keep_child_world_pose": true}
```

```json
{"type": "set_joint_axis", "joint_id": "3d0f8b2a-7c4e-4d2b-9e6f-5a1b2c3d4e5f", "axis": [0.0, 0.0, 1.0]}
```

```json
{"type": "set_joint_limits", "joint_id": "3d0f8b2a-7c4e-4d2b-9e6f-5a1b2c3d4e5f", "limits": {"lower": -1.57, "upper": 1.57, "effort": 100.0, "velocity": 1.0}}
```

## Collisions

Collision elements are addressed by `(link_id, index)` — see `describe_scene`
for links and their `collision_count`. `geometry` is one of
`{"Box": {"size": [..]}}`, `{"Cylinder": {"radius": .., "length": ..}}`,
`{"Sphere": {"radius": ..}}`, `{"Capsule": {"radius": .., "length": ..}}` or
`{"Mesh": {"path": "...", "scale": null}}`:

```json
{"type": "add_collision", "link_id": "b1c2d3e4-f5a6-4b7c-8d9e-0f1a2b3c4d5e", "geometry": {"Box": {"size": [0.2, 0.1, 0.05]}}, "origin": {"xyz": [0.0, 0.0, 0.0], "rpy": [0.0, 0.0, 0.0]}}
```

```json
{"type": "remove_collision", "link_id": "b1c2d3e4-f5a6-4b7c-8d9e-0f1a2b3c4d5e", "index": 1}
```

```json
{"type": "set_collision_origin", "link_id": "b1c2d3e4-f5a6-4b7c-8d9e-0f1a2b3c4d5e", "index": 0, "origin": {"xyz": [0.0, 0.0, 0.02], "rpy": [0.0, 0.0, 0.0]}}
```

```json
{"type": "set_collision_geometry", "link_id": "b1c2d3e4-f5a6-4b7c-8d9e-0f1a2b3c4d5e", "index": 0, "geometry": {"Sphere": {"radius": 0.06}}}
```

## Sketches

Create a sketch on a plane. A plane is `{origin, normal, x_axis, y_axis}`;
the standard planes are XY (`normal` `[0,0,1]`), XZ (`normal` `[0,1,0]`,
axes X/Z) and YZ (`normal` `[1,0,0]`, axes Y/Z):

```json
{"type": "create_sketch", "id": null, "name": "base_profile", "plane": {"origin": [0.0, 0.0, 0.0], "normal": [0.0, 0.0, 1.0], "x_axis": [1.0, 0.0, 0.0], "y_axis": [0.0, 1.0, 0.0]}}
```

```json
{"type": "delete_sketch", "sketch_id": "c9d8e7f6-a5b4-4c3d-8e2f-1a0b9c8d7e6f"}
```

Add entities **atomically** (one command = one undo step). Entities reference
each other by ID, so generate UUIDs client-side. Entity kinds:
`{"Point": {"id", "position": [x, y]}}`, `{"Line": {"id", "start", "end"}}`
(point IDs), `{"Circle": {"id", "center", "radius"}}`,
`{"Arc": {"id", "center", "start", "end", "radius"}}`.
This example is a closed 100x50 mm rectangle:

```json
{"type": "add_sketch_entities", "sketch_id": "c9d8e7f6-a5b4-4c3d-8e2f-1a0b9c8d7e6f", "entities": [
  {"Point": {"id": "11111111-1111-4111-8111-111111111111", "position": [0.0, 0.0]}},
  {"Point": {"id": "22222222-2222-4222-8222-222222222222", "position": [0.1, 0.0]}},
  {"Point": {"id": "33333333-3333-4333-8333-333333333333", "position": [0.1, 0.05]}},
  {"Point": {"id": "44444444-4444-4444-8444-444444444444", "position": [0.0, 0.05]}},
  {"Line": {"id": "55555555-5555-4555-8555-555555555555", "start": "11111111-1111-4111-8111-111111111111", "end": "22222222-2222-4222-8222-222222222222"}},
  {"Line": {"id": "66666666-6666-4666-8666-666666666666", "start": "22222222-2222-4222-8222-222222222222", "end": "33333333-3333-4333-8333-333333333333"}},
  {"Line": {"id": "77777777-7777-4777-8777-777777777777", "start": "33333333-3333-4333-8333-333333333333", "end": "44444444-4444-4444-8444-444444444444"}},
  {"Line": {"id": "88888888-8888-4888-8888-888888888888", "start": "44444444-4444-4444-8444-444444444444", "end": "11111111-1111-4111-8111-111111111111"}}
]}
```

A circle profile in one command:

```json
{"type": "add_sketch_entities", "sketch_id": "c9d8e7f6-a5b4-4c3d-8e2f-1a0b9c8d7e6f", "entities": [
  {"Point": {"id": "99999999-9999-4999-8999-999999999999", "position": [0.0, 0.0]}},
  {"Circle": {"id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", "center": "99999999-9999-4999-8999-999999999999", "radius": 0.04}}
]}
```

Replace an entity with the same ID (move a point, resize a circle) / delete
entities:

```json
{"type": "update_sketch_entity", "sketch_id": "c9d8e7f6-a5b4-4c3d-8e2f-1a0b9c8d7e6f", "entity": {"Point": {"id": "11111111-1111-4111-8111-111111111111", "position": [0.01, 0.0]}}}
```

```json
{"type": "delete_sketch_entities", "sketch_id": "c9d8e7f6-a5b4-4c3d-8e2f-1a0b9c8d7e6f", "entity_ids": ["aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"]}
```

Constraints (solved by `solve_sketch`). Geometric:
`Coincident {point1, point2}`, `Horizontal {line}`, `Vertical {line}`,
`Parallel {line1, line2}`, `Perpendicular {line1, line2}`,
`Tangent {curve1, curve2}` (one of them a circle or arc),
`EqualLength {line1, line2}`, `EqualRadius {circle1, circle2}`,
`PointOnCurve {point, curve}`, `Midpoint {point, line}`,
`Symmetric {entity1, entity2, axis}`, `Fixed {point, x, y}`. Dimensional:
`Distance {entity1, entity2, value}`,
`HorizontalDistance {point1, point2, value}` (signed, `point2 - point1`),
`VerticalDistance {point1, point2, value}`, `Angle {line1, line2, value}`
(radians, measured the shortest way round),
`Radius {circle, value}`, `Diameter {circle, value}`, `Length {line, value}`.
Lengths are in meters. All take an `id` (may be a fresh UUID); constraints are
keyed by it, so re-sending one with the same `id` and a new `value` edits the
dimension instead of adding a second one:

```json
{"type": "add_sketch_constraint", "sketch_id": "c9d8e7f6-a5b4-4c3d-8e2f-1a0b9c8d7e6f", "constraint": {"Horizontal": {"id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb", "line": "55555555-5555-4555-8555-555555555555"}}}
```

```json
{"type": "add_sketch_constraint", "sketch_id": "c9d8e7f6-a5b4-4c3d-8e2f-1a0b9c8d7e6f", "constraint": {"Distance": {"id": "cccccccc-cccc-4ccc-8ccc-cccccccccccc", "entity1": "11111111-1111-4111-8111-111111111111", "entity2": "22222222-2222-4222-8222-222222222222", "value": 0.1}}}
```

```json
{"type": "delete_sketch_constraint", "sketch_id": "c9d8e7f6-a5b4-4c3d-8e2f-1a0b9c8d7e6f", "constraint_id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"}
```

```json
{"type": "solve_sketch", "sketch_id": "c9d8e7f6-a5b4-4c3d-8e2f-1a0b9c8d7e6f"}
```

Construction geometry guides a sketch without enclosing anything — centrelines,
revolve axes, the circle a polygon is inscribed in. It is excluded from region
extraction, so it never becomes a face:

```json
{"type": "set_sketch_construction", "sketch_id": "c9d8e7f6-a5b4-4c3d-8e2f-1a0b9c8d7e6f", "entity_ids": ["aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"], "construction": true}
```

## Features (3D solids)

Extrude the regions a sketch encloses. `direction` is `"Positive" | "Negative" |
"Symmetric"` (relative to the sketch plane normal); `boolean_op` is `"New" |
"Join" | "Cut" | "Intersect"` (`target_body` required for anything but New;
note: the Truck kernel currently rejects Cut). Emits `feature_added` and
`bodies_rebuilt` with the resulting body IDs.

`profiles` names the regions to use — the IDs `describe_scene` reports for the
sketch. Leave it empty to take every region the sketch encloses. A region's
holes are cut out of the face before the sweep, so a circle drawn inside a
rectangle extrudes to a plate with a hole rather than two overlapping solids:

```json
{"type": "add_extrude", "id": null, "name": "base_pad", "sketch_id": "c9d8e7f6-a5b4-4c3d-8e2f-1a0b9c8d7e6f", "profiles": [], "distance": 0.02, "direction": "Positive", "boolean_op": "New", "target_body": null}
```

Revolve regions around an axis (angle in radians, 6.2831853 = full turn):

```json
{"type": "add_revolve", "id": null, "name": "hub", "sketch_id": "c9d8e7f6-a5b4-4c3d-8e2f-1a0b9c8d7e6f", "profiles": ["7d3a1c5e-9b2f-8e14-a6d0-3f8c1b4e7a29"], "axis_origin": [0.0, 0.0, 0.0], "axis_direction": [0.0, 0.0, 1.0], "angle": 6.2831853, "boolean_op": "New", "target_body": null}
```

Manage the feature history (rollback re-computes bodies as of just after the
given feature; `feature_id: null` rolls forward to the end):

```json
{"type": "delete_feature", "feature_id": "d1e2f3a4-b5c6-4d7e-8f9a-0b1c2d3e4f5a"}
```

```json
{"type": "set_feature_suppressed", "feature_id": "d1e2f3a4-b5c6-4d7e-8f9a-0b1c2d3e4f5a", "suppressed": true}
```

```json
{"type": "rollback_to", "feature_id": "d1e2f3a4-b5c6-4d7e-8f9a-0b1c2d3e4f5a"}
```

```json
{"type": "rollback_to", "feature_id": null}
```

```json
{"type": "rebuild_features"}
```

## History

```json
{"type": "undo"}
```

```json
{"type": "redo"}
```
