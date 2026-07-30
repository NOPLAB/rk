//! The webview builds engine commands as JSON in TypeScript
//! (`apps/desktop/src/engine/commands.ts`). Nothing on the Rust side checks
//! those field names, so a typo would only surface as a runtime error in the
//! app. These tests apply the exact payloads the UI sends.

use std::sync::Arc;

use rk_engine::{Command, Engine, Event};
use serde_json::{Value, json};
use uuid::Uuid;

fn engine() -> Engine {
    Engine::new(Arc::from(rk_cad::default_kernel()))
}

/// Deserialize a UI payload and apply it, as `engine_apply` does
fn apply(eng: &mut Engine, payload: Value) -> Vec<Event> {
    let cmd: Command =
        serde_json::from_value(payload.clone()).unwrap_or_else(|e| panic!("{payload}: {e}"));
    eng.apply(cmd).unwrap_or_else(|e| panic!("{payload}: {e}"))
}

/// `standardPlane("XY")`
fn xy_plane() -> Value {
    json!({
        "origin": [0.0, 0.0, 0.0],
        "normal": [0.0, 0.0, 1.0],
        "x_axis": [1.0, 0.0, 0.0],
        "y_axis": [0.0, 1.0, 0.0],
    })
}

fn create_sketch(eng: &mut Engine) -> Uuid {
    let events = apply(
        eng,
        json!({
            "type": "create_sketch",
            "id": null,
            "name": "Sketch 1 (XY)",
            "plane": xy_plane(),
        }),
    );
    events
        .iter()
        .find_map(|e| match e {
            Event::SketchAdded { sketch_id } => Some(*sketch_id),
            _ => None,
        })
        .expect("SketchAdded event")
}

/// The rectangle tool: four points and four lines sharing point IDs, in a
/// single command — that sharing is what makes the profile closed.
/// Returns the corner points and the edges, counter-clockwise from the origin.
fn rectangle(eng: &mut Engine, sketch_id: Uuid) -> (Vec<Uuid>, Vec<Uuid>) {
    let corners = [[0.0, 0.0], [0.05, 0.0], [0.05, 0.03], [0.0, 0.03]];
    let points: Vec<Uuid> = (0..4).map(|_| Uuid::new_v4()).collect();
    let lines: Vec<Uuid> = (0..4).map(|_| Uuid::new_v4()).collect();
    let mut entities: Vec<Value> = points
        .iter()
        .zip(corners)
        .map(|(id, position)| json!({"Point": {"id": id, "position": position}}))
        .collect();
    for i in 0..4 {
        entities.push(json!({"Line": {
            "id": lines[i],
            "start": points[i],
            "end": points[(i + 1) % 4],
        }}));
    }
    apply(
        eng,
        json!({
            "type": "add_sketch_entities",
            "sketch_id": sketch_id,
            "entities": entities,
        }),
    );
    (points, lines)
}

/// `sketchPoint` for a standalone line: two points plus the line between them
fn line(eng: &mut Engine, sketch_id: Uuid, from: [f32; 2], to: [f32; 2]) -> (Uuid, Uuid, Uuid) {
    let (a, b, id) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    apply(
        eng,
        json!({
            "type": "add_sketch_entities",
            "sketch_id": sketch_id,
            "entities": [
                {"Point": {"id": a, "position": from}},
                {"Point": {"id": b, "position": to}},
                {"Line": {"id": id, "start": a, "end": b}},
            ],
        }),
    );
    (a, b, id)
}

fn point_position(eng: &Engine, sketch_id: Uuid, point_id: Uuid) -> glam::Vec2 {
    match eng.sketch(sketch_id).unwrap().get_entity(point_id) {
        Some(rk_cad::SketchEntity::Point { position, .. }) => *position,
        other => panic!("{point_id} is not a point: {other:?}"),
    }
}

fn constraint_count(eng: &Engine, sketch_id: Uuid) -> usize {
    eng.sketch(sketch_id).unwrap().constraints().len()
}

/// Two connected boxes, returning the child part and its link. Collisions
/// hang off links, and links only exist for parts in the assembly.
fn connected_pair(eng: &mut Engine) -> (Uuid, Uuid) {
    let mut make_box = || {
        let events = apply(
            eng,
            json!({
                "type": "create_primitive",
                "id": null,
                "primitive": {"shape": "box", "size": [0.1, 0.1, 0.1]},
                "name": null,
            }),
        );
        events
            .iter()
            .find_map(|e| match e {
                Event::PartAdded { part_id } => Some(*part_id),
                _ => None,
            })
            .expect("PartAdded event")
    };
    let parent = make_box();
    let child = make_box();
    apply(
        eng,
        json!({"type": "connect_parts", "parent_part": parent, "child_part": child}),
    );
    let link_id = eng
        .assembly()
        .links
        .values()
        .find(|l| l.part_id == Some(child))
        .expect("child link")
        .id;
    (child, link_id)
}

#[test]
fn collision_payloads_apply() {
    let mut eng = engine();
    let (_part, link_id) = connected_pair(&mut eng);
    let collisions = |eng: &Engine| eng.assembly().links[&link_id].collisions.len();
    // `Link::from_part` seeds a placeholder box, so a fresh link is not empty
    let base = collisions(&eng);
    assert_eq!(base, 1);

    for geometry in [
        json!({"Box": {"size": [0.05, 0.05, 0.05]}}),
        json!({"Cylinder": {"radius": 0.025, "length": 0.05}}),
        json!({"Sphere": {"radius": 0.025}}),
        json!({"Capsule": {"radius": 0.02, "length": 0.05}}),
    ] {
        apply(
            &mut eng,
            json!({
                "type": "add_collision",
                "link_id": link_id,
                "geometry": geometry,
                "origin": {"xyz": [0.0, 0.0, 0.0], "rpy": [0.0, 0.0, 0.0]},
            }),
        );
    }
    assert_eq!(collisions(&eng), base + 4);

    apply(
        &mut eng,
        json!({
            "type": "set_collision_origin",
            "link_id": link_id,
            "index": base,
            "origin": {"xyz": [0.0, 0.0, 0.02], "rpy": [0.0, 1.5707964, 0.0]},
        }),
    );
    apply(
        &mut eng,
        json!({
            "type": "set_collision_geometry",
            "link_id": link_id,
            "index": base + 1,
            "geometry": {"Sphere": {"radius": 0.06}},
        }),
    );
    apply(
        &mut eng,
        json!({"type": "remove_collision", "link_id": link_id, "index": base + 3}),
    );
    assert_eq!(collisions(&eng), base + 3);

    let link = &eng.assembly().links[&link_id];
    assert_eq!(link.collisions[base].origin.xyz, [0.0, 0.0, 0.02]);
    assert!(matches!(
        link.collisions[base + 1].geometry,
        rk_core::GeometryType::Sphere { radius } if (radius - 0.06).abs() < 1e-6
    ));
}

#[test]
fn physics_payloads_apply() {
    let mut eng = engine();
    let (part_id, _link) = connected_pair(&mut eng);

    apply(
        &mut eng,
        json!({"type": "set_part_mass", "part_id": part_id, "mass": 1.5}),
    );
    apply(
        &mut eng,
        json!({
            "type": "set_part_inertia",
            "part_id": part_id,
            "inertia": {
                "ixx": 0.001, "ixy": 0.0, "ixz": 0.0,
                "iyy": 0.002, "iyz": 0.0, "izz": 0.003,
            },
        }),
    );

    let part = eng.part(part_id).expect("part exists");
    assert_eq!(part.mass, 1.5);
    assert_eq!(part.inertia.izz, 0.003);

    // URDF export writes the link's copy, so it has to track the part
    let link = eng
        .assembly()
        .links
        .values()
        .find(|l| l.part_id == Some(part_id))
        .expect("link exists");
    assert_eq!(link.inertial.mass, 1.5);
    assert_eq!(link.inertial.inertia.izz, 0.003);
}

/// Every payload shape `src/engine/constraints.ts` can emit
#[test]
fn constraint_payloads_apply() {
    let mut eng = engine();
    let sketch_id = create_sketch(&mut eng);
    let (points, lines) = rectangle(&mut eng, sketch_id);
    let center = Uuid::new_v4();
    let circle = Uuid::new_v4();
    let hole = Uuid::new_v4();
    apply(
        &mut eng,
        json!({
            "type": "add_sketch_entities",
            "sketch_id": sketch_id,
            "entities": [
                {"Point": {"id": center, "position": [0.025, 0.015]}},
                {"Circle": {"id": circle, "center": center, "radius": 0.006}},
                {"Point": {"id": hole, "position": [0.1, 0.0]}},
                {"Circle": {"id": Uuid::new_v4(), "center": hole, "radius": 0.004}},
            ],
        }),
    );
    let circle2 = match eng
        .sketch(sketch_id)
        .unwrap()
        .entities()
        .values()
        .find(|e| matches!(e, rk_cad::SketchEntity::Circle { center, .. } if *center == hole))
    {
        Some(e) => e.id(),
        None => panic!("second circle"),
    };

    let constraints = [
        json!({"Horizontal": {"id": Uuid::new_v4(), "line": lines[0]}}),
        json!({"Vertical": {"id": Uuid::new_v4(), "line": lines[1]}}),
        json!({"Parallel": {"id": Uuid::new_v4(), "line1": lines[0], "line2": lines[2]}}),
        json!({"Perpendicular": {"id": Uuid::new_v4(), "line1": lines[0], "line2": lines[3]}}),
        json!({"EqualLength": {"id": Uuid::new_v4(), "line1": lines[0], "line2": lines[2]}}),
        json!({"EqualRadius": {"id": Uuid::new_v4(), "circle1": circle, "circle2": circle2}}),
        json!({"Tangent": {"id": Uuid::new_v4(), "curve1": circle, "curve2": lines[0]}}),
        json!({"PointOnCurve": {"id": Uuid::new_v4(), "point": center, "curve": lines[1]}}),
        json!({"Midpoint": {"id": Uuid::new_v4(), "point": center, "line": lines[2]}}),
        json!({"Coincident": {"id": Uuid::new_v4(), "point1": points[0], "point2": points[1]}}),
        json!({"Fixed": {"id": Uuid::new_v4(), "point": points[0], "x": 0.0, "y": 0.0}}),
        json!({"Length": {"id": Uuid::new_v4(), "line": lines[0], "value": 0.05}}),
        json!({"Distance": {"id": Uuid::new_v4(), "entity1": points[0], "entity2": points[2], "value": 0.058}}),
        json!({"HorizontalDistance": {"id": Uuid::new_v4(), "point1": points[0], "point2": points[1], "value": 0.05}}),
        json!({"VerticalDistance": {"id": Uuid::new_v4(), "point1": points[1], "point2": points[2], "value": 0.03}}),
        json!({"Angle": {"id": Uuid::new_v4(), "line1": lines[0], "line2": lines[1], "value": 1.5707964}}),
        json!({"Radius": {"id": Uuid::new_v4(), "circle": circle, "value": 0.006}}),
        json!({"Diameter": {"id": Uuid::new_v4(), "circle": circle2, "value": 0.008}}),
    ];
    let expected = constraints.len();
    for constraint in constraints {
        apply(
            &mut eng,
            json!({
                "type": "add_sketch_constraint",
                "sketch_id": sketch_id,
                "constraint": constraint,
            }),
        );
    }
    assert_eq!(constraint_count(&eng, sketch_id), expected);

    // Solving with the whole set applied must not error out
    apply(
        &mut eng,
        json!({"type": "solve_sketch", "sketch_id": sketch_id}),
    );

    let victim = eng
        .sketch(sketch_id)
        .unwrap()
        .constraints_iter()
        .next()
        .unwrap()
        .id();
    apply(
        &mut eng,
        json!({
            "type": "delete_sketch_constraint",
            "sketch_id": sketch_id,
            "constraint_id": victim,
        }),
    );
    assert_eq!(constraint_count(&eng, sketch_id), expected - 1);
}

/// The dimension list edits a value by re-sending the constraint with the same
/// ID, which is only an edit because constraints are keyed by ID
#[test]
fn dimension_edit_replaces_the_constraint() {
    let mut eng = engine();
    let sketch_id = create_sketch(&mut eng);
    let (a, b, line_id) = line(&mut eng, sketch_id, [0.0, 0.0], [0.04, 0.0]);
    let constraint_id = Uuid::new_v4();

    let set_length = |eng: &mut Engine, value: f32| {
        apply(
            eng,
            json!({
                "type": "add_sketch_constraint",
                "sketch_id": sketch_id,
                "constraint": {"Length": {"id": constraint_id, "line": line_id, "value": value}},
            }),
        );
        apply(eng, json!({"type": "solve_sketch", "sketch_id": sketch_id}));
        (point_position(eng, sketch_id, b) - point_position(eng, sketch_id, a)).length()
    };

    assert!((set_length(&mut eng, 0.08) - 0.08).abs() < 1e-3);
    assert!((set_length(&mut eng, 0.06) - 0.06).abs() < 1e-3);
    assert_eq!(
        constraint_count(&eng, sketch_id),
        1,
        "the same ID replaces instead of adding"
    );
}

/// Radius is not a solver variable, so this only works because the solver
/// assigns radius dimensions directly
#[test]
fn radius_payload_drives_the_circle() {
    let mut eng = engine();
    let sketch_id = create_sketch(&mut eng);
    let (center, circle) = (Uuid::new_v4(), Uuid::new_v4());
    apply(
        &mut eng,
        json!({
            "type": "add_sketch_entities",
            "sketch_id": sketch_id,
            "entities": [
                {"Point": {"id": center, "position": [0.0, 0.0]}},
                {"Circle": {"id": circle, "center": center, "radius": 0.005}},
            ],
        }),
    );
    apply(
        &mut eng,
        json!({
            "type": "add_sketch_constraint",
            "sketch_id": sketch_id,
            "constraint": {"Radius": {"id": Uuid::new_v4(), "circle": circle, "value": 0.012}},
        }),
    );
    apply(
        &mut eng,
        json!({"type": "solve_sketch", "sketch_id": sketch_id}),
    );

    let radius = match eng.sketch(sketch_id).unwrap().get_entity(circle) {
        Some(rk_cad::SketchEntity::Circle { radius, .. }) => *radius,
        other => panic!("not a circle: {other:?}"),
    };
    assert!((radius - 0.012).abs() < 1e-6, "radius is {radius}");
}

#[test]
fn sketch_payloads_apply() {
    let mut eng = engine();
    let sketch_id = create_sketch(&mut eng);
    let (points, _lines) = rectangle(&mut eng, sketch_id);

    // The circle tool: a center point plus the circle referencing it
    let center = Uuid::new_v4();
    apply(
        &mut eng,
        json!({
            "type": "add_sketch_entities",
            "sketch_id": sketch_id,
            "entities": [
                {"Point": {"id": center, "position": [0.2, 0.0]}},
                {"Circle": {"id": Uuid::new_v4(), "center": center, "radius": 0.02}},
            ],
        }),
    );

    apply(
        &mut eng,
        json!({"type": "solve_sketch", "sketch_id": sketch_id}),
    );

    let sketch = eng.sketch(sketch_id).expect("sketch exists");
    assert_eq!(sketch.entities().len(), 4 + 4 + 2);
    // Rectangle and circle are both closed profiles
    assert_eq!(sketch.extract_profiles().unwrap().len(), 2);

    // Delete key on a selected entity
    apply(
        &mut eng,
        json!({
            "type": "delete_sketch_entities",
            "sketch_id": sketch_id,
            "entity_ids": [points[0]],
        }),
    );
    assert!(
        eng.sketch(sketch_id)
            .unwrap()
            .get_entity(points[0])
            .is_none()
    );

    apply(
        &mut eng,
        json!({"type": "delete_sketch", "sketch_id": sketch_id}),
    );
    assert!(eng.sketch(sketch_id).is_none());
}

/// The curve tools beyond line/rectangle/circle, plus the construction toggle
#[test]
fn curve_and_construction_payloads_apply() {
    let mut eng = engine();
    let sketch_id = create_sketch(&mut eng);

    let (centre, start, end, arc) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let (ellipse_centre, ellipse) = (Uuid::new_v4(), Uuid::new_v4());
    let knots: Vec<Uuid> = (0..4).map(|_| Uuid::new_v4()).collect();
    let spline = Uuid::new_v4();

    apply(
        &mut eng,
        json!({
            "type": "add_sketch_entities",
            "sketch_id": sketch_id,
            "entities": [
                {"Point": {"id": centre, "position": [0.0, 0.0]}},
                {"Point": {"id": start, "position": [0.02, 0.0]}},
                {"Point": {"id": end, "position": [0.0, 0.02]}},
                {"Arc": {"id": arc, "center": centre, "start": start, "end": end, "radius": 0.02}},
                {"Point": {"id": ellipse_centre, "position": [0.1, 0.0]}},
                {"Ellipse": {
                    "id": ellipse,
                    "center": ellipse_centre,
                    "major_radius": 0.03,
                    "minor_radius": 0.015,
                    "rotation": 0.5,
                }},
                {"Point": {"id": knots[0], "position": [0.2, 0.0]}},
                {"Point": {"id": knots[1], "position": [0.22, 0.02]}},
                {"Point": {"id": knots[2], "position": [0.25, 0.0]}},
                {"Point": {"id": knots[3], "position": [0.22, -0.02]}},
                {"Spline": {"id": spline, "control_points": knots, "closed": true}},
            ],
        }),
    );

    let sketch = eng.sketch(sketch_id).expect("sketch exists");
    assert!(matches!(
        sketch.get_entity(arc),
        Some(rk_cad::SketchEntity::Arc { .. })
    ));
    assert!(matches!(
        sketch.get_entity(ellipse),
        Some(rk_cad::SketchEntity::Ellipse { .. })
    ));
    assert!(matches!(
        sketch.get_entity(spline),
        Some(rk_cad::SketchEntity::Spline { .. })
    ));
    // The ellipse and the closed spline each enclose an area
    assert!(
        sketch.profiles().len() >= 2,
        "closed curves enclose regions: {:?}",
        sketch.profiles().len(),
    );

    apply(
        &mut eng,
        json!({
            "type": "set_sketch_construction",
            "sketch_id": sketch_id,
            "entity_ids": [ellipse],
            "construction": true,
        }),
    );
    assert!(eng.sketch(sketch_id).unwrap().is_construction(ellipse));

    apply(
        &mut eng,
        json!({
            "type": "set_sketch_construction",
            "sketch_id": sketch_id,
            "entity_ids": [ellipse],
            "construction": false,
        }),
    );
    assert!(!eng.sketch(sketch_id).unwrap().is_construction(ellipse));
}

/// Sketching on a solid's face sends a frame that is not axis-aligned
#[test]
fn a_face_derived_sketch_plane_applies() {
    let mut eng = engine();
    // A 45° plane, the way `planeFromHit` builds one from a picked face
    let events = apply(
        &mut eng,
        json!({
            "type": "create_sketch",
            "id": null,
            "name": "Sketch on face",
            "plane": {
                "origin": [0.01, 0.02, 0.03],
                "normal": [0.0, 0.70710677, 0.70710677],
                "x_axis": [1.0, 0.0, 0.0],
                "y_axis": [0.0, 0.70710677, -0.70710677],
            },
        }),
    );
    let sketch_id = events
        .iter()
        .find_map(|e| match e {
            Event::SketchAdded { sketch_id } => Some(*sketch_id),
            _ => None,
        })
        .expect("SketchAdded event");
    let plane = eng.sketch(sketch_id).unwrap().plane;
    assert!((plane.normal.y - 0.70710677).abs() < 1e-6);
    assert!((plane.origin.z - 0.03).abs() < 1e-6);
}

#[test]
fn feature_payloads_apply() {
    if !rk_cad::default_kernel().is_available() {
        return;
    }
    let mut eng = engine();
    let sketch_id = create_sketch(&mut eng);
    rectangle(&mut eng, sketch_id);
    // The dialog sends the clicked region; an empty list would mean "all"
    let region = eng.sketch(sketch_id).unwrap().profiles()[0].id;

    let events = apply(
        &mut eng,
        json!({
            "type": "add_extrude",
            "id": null,
            "name": "Extrude 1",
            "sketch_id": sketch_id,
            "profiles": [region],
            "distance": 0.01,
            "direction": "Positive",
            "boolean_op": "New",
            "target_body": null,
        }),
    );
    let feature_id = events
        .iter()
        .find_map(|e| match e {
            Event::FeatureAdded { feature_id } => Some(*feature_id),
            _ => None,
        })
        .expect("FeatureAdded event");
    assert_eq!(eng.body_ids().len(), 1);

    apply(
        &mut eng,
        json!({
            "type": "set_feature_suppressed",
            "feature_id": feature_id,
            "suppressed": true,
        }),
    );
    assert!(
        eng.body_ids().is_empty(),
        "suppressed feature builds nothing"
    );
    apply(
        &mut eng,
        json!({
            "type": "set_feature_suppressed",
            "feature_id": feature_id,
            "suppressed": false,
        }),
    );

    apply(
        &mut eng,
        json!({"type": "rollback_to", "feature_id": feature_id}),
    );
    apply(&mut eng, json!({"type": "rollback_to", "feature_id": null}));
    assert_eq!(eng.body_ids().len(), 1);

    apply(
        &mut eng,
        json!({"type": "rename_feature", "feature_id": feature_id, "name": "Base Pad"}),
    );
    assert_eq!(
        eng.document()
            .cad
            .history
            .get_by_id(feature_id)
            .unwrap()
            .name(),
        "Base Pad"
    );

    apply(
        &mut eng,
        json!({"type": "delete_feature", "feature_id": feature_id}),
    );
    assert!(eng.body_ids().is_empty());
}

/// The browser's Group / Rename / Collapse / Ungroup menu items
#[test]
fn feature_group_payloads_apply() {
    if !rk_cad::default_kernel().is_available() {
        return;
    }
    let mut eng = engine();
    let sketch_id = create_sketch(&mut eng);
    rectangle(&mut eng, sketch_id);

    let mut feature_ids = Vec::new();
    for (i, distance) in [0.01_f32, 0.02].iter().enumerate() {
        let events = apply(
            &mut eng,
            json!({
                "type": "add_extrude",
                "id": null,
                "name": format!("Extrude {}", i + 1),
                "sketch_id": sketch_id,
                "profiles": [],
                "distance": distance,
                "direction": "Positive",
                "boolean_op": "New",
                "target_body": null,
            }),
        );
        feature_ids.push(
            events
                .iter()
                .find_map(|e| match e {
                    Event::FeatureAdded { feature_id } => Some(*feature_id),
                    _ => None,
                })
                .expect("FeatureAdded event"),
        );
    }

    let group_id = Uuid::new_v4();
    apply(
        &mut eng,
        json!({
            "type": "group_features",
            "id": group_id,
            "name": "Pads",
            "feature_ids": feature_ids,
        }),
    );
    apply(
        &mut eng,
        json!({"type": "rename_feature_group", "group_id": group_id, "name": "Boss"}),
    );
    apply(
        &mut eng,
        json!({"type": "set_feature_group_collapsed", "group_id": group_id, "collapsed": true}),
    );
    let group = eng.document().cad.history.get_group(group_id).unwrap();
    assert_eq!(group.name, "Boss");
    assert!(group.collapsed);
    assert_eq!(group.members, feature_ids);

    apply(
        &mut eng,
        json!({"type": "ungroup_features", "group_id": group_id}),
    );
    assert!(eng.document().cad.history.groups().is_empty());
    assert_eq!(
        eng.body_ids().len(),
        2,
        "grouping and ungrouping never touch the bodies"
    );
}

/// The sketch's Rename menu item
#[test]
fn rename_sketch_payload_applies() {
    let mut eng = engine();
    let sketch_id = create_sketch(&mut eng);
    apply(
        &mut eng,
        json!({"type": "rename_sketch", "sketch_id": sketch_id, "name": "Base Profile"}),
    );
    assert_eq!(eng.sketch(sketch_id).unwrap().name, "Base Profile");
}

#[test]
fn revolve_payload_applies() {
    if !rk_cad::default_kernel().is_available() {
        return;
    }
    let mut eng = engine();
    let sketch_id = create_sketch(&mut eng);
    rectangle(&mut eng, sketch_id);

    // Revolve around world Y so the profile (on XY, off the axis) sweeps a ring
    let cmd: Command = serde_json::from_value(json!({
        "type": "add_revolve",
        "id": null,
        "name": "Revolve 1",
        "sketch_id": sketch_id,
        "profiles": [],
        "axis_origin": [0.0, 0.0, 0.0],
        "axis_direction": [0.0, 1.0, 0.0],
        "angle": std::f32::consts::TAU,
        "boolean_op": "New",
        "target_body": null,
    }))
    .expect("revolve payload is a valid command");
    // The kernel may reject the sweep; the payload shape is what matters here
    let _ = eng.apply(cmd);
}
