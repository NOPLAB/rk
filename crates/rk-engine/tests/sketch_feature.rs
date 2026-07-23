//! Sketch → solve → extrude → body mesh, end to end with the real kernel

mod common;

use glam::Vec2;
use rk_cad::{BooleanOp, ExtrudeDirection, SketchPlane};
use rk_engine::{Command, Event, ExtrudePreviewRequest};
use uuid::Uuid;

use common::{create_rect_sketch, engine};

fn kernel_available() -> bool {
    rk_cad::default_kernel().is_available()
}

#[test]
fn sketch_extrude_produces_body_mesh() {
    if !kernel_available() {
        return;
    }
    let mut eng = engine();
    let sketch_id = create_rect_sketch(&mut eng);

    eng.apply(Command::SolveSketch { sketch_id }).unwrap();

    let feature_id = Uuid::new_v4();
    let events = eng
        .apply(Command::AddExtrude {
            id: Some(feature_id),
            name: None,
            sketch_id,
            distance: 5.0,
            direction: ExtrudeDirection::Positive,
            boolean_op: BooleanOp::New,
            target_body: None,
        })
        .unwrap();

    assert!(events.contains(&Event::FeatureAdded { feature_id }));
    let body_ids = events
        .iter()
        .find_map(|e| match e {
            Event::BodiesRebuilt { body_ids } => Some(body_ids.clone()),
            _ => None,
        })
        .expect("BodiesRebuilt event");
    assert_eq!(body_ids.len(), 1);

    let mesh = eng.body_mesh(body_ids[0]).unwrap();
    assert!(!mesh.is_empty());
    assert!(mesh.triangle_count() > 0);
}

#[test]
fn delete_feature_removes_body() {
    if !kernel_available() {
        return;
    }
    let mut eng = engine();
    let sketch_id = create_rect_sketch(&mut eng);
    let feature_id = Uuid::new_v4();
    eng.apply(Command::AddExtrude {
        id: Some(feature_id),
        name: None,
        sketch_id,
        distance: 5.0,
        direction: ExtrudeDirection::Positive,
        boolean_op: BooleanOp::New,
        target_body: None,
    })
    .unwrap();
    assert_eq!(eng.body_ids().len(), 1);

    let events = eng.apply(Command::DeleteFeature { feature_id }).unwrap();
    assert!(events.contains(&Event::FeatureRemoved { feature_id }));
    assert!(eng.body_ids().is_empty());
}

#[test]
fn suppression_toggles_body() {
    if !kernel_available() {
        return;
    }
    let mut eng = engine();
    let sketch_id = create_rect_sketch(&mut eng);
    let feature_id = Uuid::new_v4();
    eng.apply(Command::AddExtrude {
        id: Some(feature_id),
        name: None,
        sketch_id,
        distance: 5.0,
        direction: ExtrudeDirection::Positive,
        boolean_op: BooleanOp::New,
        target_body: None,
    })
    .unwrap();
    let original_bodies = eng.body_ids();

    eng.apply(Command::SetFeatureSuppressed {
        feature_id,
        suppressed: true,
    })
    .unwrap();
    assert!(eng.body_ids().is_empty());

    eng.apply(Command::SetFeatureSuppressed {
        feature_id,
        suppressed: false,
    })
    .unwrap();
    assert_eq!(eng.body_ids(), original_bodies, "body ID stable");
}

#[test]
fn preview_extrude_returns_mesh_without_side_effects() {
    if !kernel_available() {
        return;
    }
    let mut eng = engine();
    let sketch_id = create_rect_sketch(&mut eng);
    let revision = eng.revision();

    let profiles = eng.sketch(sketch_id).unwrap().extract_profiles().unwrap();
    assert!(!profiles.is_empty());

    let mesh = eng
        .preview_extrude(&ExtrudePreviewRequest {
            sketch_id,
            profiles,
            distance: 5.0,
            direction: ExtrudeDirection::Positive,
        })
        .unwrap();
    assert!(!mesh.is_empty());

    assert_eq!(eng.revision(), revision, "preview is a pure query");
    assert!(eng.body_ids().is_empty());
    assert!(!eng.can_undo() || eng.undo_description() != Some("Extrude"));
}

#[test]
fn solve_with_dimension_constraint_moves_points() {
    if !kernel_available() {
        return;
    }
    let mut eng = engine();
    let sketch_id = Uuid::new_v4();
    eng.apply(Command::CreateSketch {
        id: Some(sketch_id),
        name: None,
        plane: SketchPlane::xy(),
    })
    .unwrap();

    // Two points 1.0 apart, constrained to distance 2.0
    let mut tmp = rk_cad::Sketch::new("tmp", SketchPlane::xy());
    let a = tmp.add_point(Vec2::ZERO);
    let b = tmp.add_point(Vec2::new(1.0, 0.0));
    let entities: Vec<_> = tmp.entities().values().cloned().collect();
    eng.apply(Command::AddSketchEntities {
        sketch_id,
        entities,
    })
    .unwrap();
    eng.apply(Command::AddSketchConstraint {
        sketch_id,
        constraint: rk_cad::SketchConstraint::distance(a, b, 2.0),
    })
    .unwrap();
    eng.apply(Command::SolveSketch { sketch_id }).unwrap();

    let sketch = eng.sketch(sketch_id).unwrap();
    let pa = sketch.get_entity(a).unwrap();
    let pb = sketch.get_entity(b).unwrap();
    let (pa, pb) = match (pa, pb) {
        (
            rk_cad::SketchEntity::Point { position: p1, .. },
            rk_cad::SketchEntity::Point { position: p2, .. },
        ) => (*p1, *p2),
        other => panic!("expected points, got {other:?}"),
    };
    assert!(
        ((pa - pb).length() - 2.0).abs() < 1e-3,
        "solver enforced the distance constraint: {} vs 2.0",
        (pa - pb).length()
    );
}
