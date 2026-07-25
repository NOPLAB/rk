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
/// single command — that sharing is what makes the profile closed
fn rectangle(eng: &mut Engine, sketch_id: Uuid) -> Vec<Uuid> {
    let corners = [[0.0, 0.0], [0.05, 0.0], [0.05, 0.03], [0.0, 0.03]];
    let points: Vec<Uuid> = (0..4).map(|_| Uuid::new_v4()).collect();
    let mut entities: Vec<Value> = points
        .iter()
        .zip(corners)
        .map(|(id, position)| json!({"Point": {"id": id, "position": position}}))
        .collect();
    for i in 0..4 {
        entities.push(json!({"Line": {
            "id": Uuid::new_v4(),
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
    points
}

#[test]
fn sketch_payloads_apply() {
    let mut eng = engine();
    let sketch_id = create_sketch(&mut eng);
    let points = rectangle(&mut eng, sketch_id);

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

#[test]
fn feature_payloads_apply() {
    if !rk_cad::default_kernel().is_available() {
        return;
    }
    let mut eng = engine();
    let sketch_id = create_sketch(&mut eng);
    rectangle(&mut eng, sketch_id);

    let events = apply(
        &mut eng,
        json!({
            "type": "add_extrude",
            "id": null,
            "name": "Extrude 1",
            "sketch_id": sketch_id,
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
        json!({"type": "delete_feature", "feature_id": feature_id}),
    );
    assert!(eng.body_ids().is_empty());
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
