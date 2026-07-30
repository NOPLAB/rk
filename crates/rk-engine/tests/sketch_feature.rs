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
            profiles: Vec::new(),
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
        profiles: Vec::new(),
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
        profiles: Vec::new(),
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

/// Two separate squares: extruding one of them must not build the other
#[test]
fn extrude_uses_only_the_selected_region() {
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

    let mut tmp = rk_cad::Sketch::new("tmp", SketchPlane::xy());
    for origin in [Vec2::ZERO, Vec2::new(10.0, 0.0)] {
        let corners = [
            origin,
            origin + Vec2::new(2.0, 0.0),
            origin + Vec2::new(2.0, 2.0),
            origin + Vec2::new(0.0, 2.0),
        ];
        let ids: Vec<Uuid> = corners.iter().map(|c| tmp.add_point(*c)).collect();
        for i in 0..4 {
            tmp.add_line(ids[i], ids[(i + 1) % 4]);
        }
    }
    let entities: Vec<_> = tmp.entities().values().cloned().collect();
    eng.apply(Command::AddSketchEntities {
        sketch_id,
        entities,
    })
    .unwrap();

    let regions = eng.sketch(sketch_id).unwrap().profiles();
    assert_eq!(regions.len(), 2, "two squares, two regions");

    eng.apply(Command::AddExtrude {
        id: None,
        name: None,
        sketch_id,
        profiles: vec![regions[0].id],
        distance: 1.0,
        direction: ExtrudeDirection::Positive,
        boolean_op: BooleanOp::New,
        target_body: None,
    })
    .unwrap();

    let body = eng.body_ids()[0];
    let mesh = eng.body_mesh(body).unwrap().clone();
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for v in &mesh.vertices {
        lo = lo.min(v[0]);
        hi = hi.max(v[0]);
    }
    assert!(
        hi - lo < 3.0,
        "only one square was extruded, but the body spans {} in x",
        hi - lo,
    );
}

/// A circle inside a square is a hole in it, not a second solid
#[test]
fn extrude_cuts_the_holes_out_of_the_region() {
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

    let mut tmp = rk_cad::Sketch::new("tmp", SketchPlane::xy());
    let corners = [
        Vec2::ZERO,
        Vec2::new(4.0, 0.0),
        Vec2::new(4.0, 4.0),
        Vec2::new(0.0, 4.0),
    ];
    let ids: Vec<Uuid> = corners.iter().map(|c| tmp.add_point(*c)).collect();
    for i in 0..4 {
        tmp.add_line(ids[i], ids[(i + 1) % 4]);
    }
    let centre = tmp.add_point(Vec2::new(2.0, 2.0));
    tmp.add_circle(centre, 1.0);
    let entities: Vec<_> = tmp.entities().values().cloned().collect();
    eng.apply(Command::AddSketchEntities {
        sketch_id,
        entities,
    })
    .unwrap();

    let regions = eng.sketch(sketch_id).unwrap().profiles();
    assert_eq!(regions.len(), 2, "the plate and the disc");
    let plate = &regions[0];
    assert_eq!(plate.holes.len(), 1, "the disc is a hole in the plate");

    eng.apply(Command::AddExtrude {
        id: None,
        name: None,
        sketch_id,
        profiles: vec![plate.id],
        distance: 1.0,
        direction: ExtrudeDirection::Positive,
        boolean_op: BooleanOp::New,
        target_body: None,
    })
    .unwrap();

    let body = eng.body_ids()[0];
    let mesh = eng.body_mesh(body).unwrap().clone();
    let volume: f32 = mesh
        .indices
        .chunks(3)
        .filter_map(|tri| match tri {
            [i, j, k] => {
                let at = |n: &u32| glam::Vec3::from_array(mesh.vertices[*n as usize]);
                Some(at(i).dot(at(j).cross(at(k))) / 6.0)
            }
            _ => None,
        })
        .sum();
    let want = 16.0 - std::f32::consts::PI;
    assert!(
        (volume.abs() - want).abs() / want < 0.05,
        "plate volume {} is not {want} — the hole was dropped",
        volume.abs(),
    );
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

#[test]
fn grouping_features_never_changes_the_model() {
    if !kernel_available() {
        return;
    }
    let mut eng = engine();
    let sketch_id = create_rect_sketch(&mut eng);

    let mut feature_ids = Vec::new();
    for distance in [5.0_f32, 8.0] {
        let id = Uuid::new_v4();
        eng.apply(Command::AddExtrude {
            id: Some(id),
            name: None,
            sketch_id,
            profiles: Vec::new(),
            distance,
            direction: ExtrudeDirection::Positive,
            boolean_op: BooleanOp::New,
            target_body: None,
        })
        .unwrap();
        feature_ids.push(id);
    }
    let bodies_before = eng.body_ids().len();

    let group_id = Uuid::new_v4();
    let events = eng
        .apply(Command::GroupFeatures {
            id: Some(group_id),
            name: Some("Pads".into()),
            feature_ids: feature_ids.clone(),
        })
        .unwrap();
    assert!(events.contains(&Event::FeatureGroupsChanged));
    assert_eq!(
        eng.body_ids().len(),
        bodies_before,
        "grouping is presentation only"
    );

    let groups = eng.document().cad.history.groups();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].members, feature_ids);

    // Deleting a member prunes it; deleting the last one takes the group
    eng.apply(Command::DeleteFeature {
        feature_id: feature_ids[0],
    })
    .unwrap();
    assert_eq!(
        eng.document().cad.history.groups()[0].members,
        vec![feature_ids[1]]
    );
    eng.apply(Command::DeleteFeature {
        feature_id: feature_ids[1],
    })
    .unwrap();
    assert!(eng.document().cad.history.groups().is_empty());
}

#[test]
fn grouping_rejects_a_feature_that_does_not_exist() {
    let mut eng = engine();
    let err = eng
        .apply(Command::GroupFeatures {
            id: None,
            name: None,
            feature_ids: vec![Uuid::new_v4()],
        })
        .unwrap_err();
    assert!(err.to_string().contains("feature"), "{err}");
}
