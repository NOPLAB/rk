//! Undo/redo, interactive coalescing, and failure atomicity

mod common;

use glam::{Mat4, Vec3};
use rk_engine::{Command, Event, ResetReason};
use uuid::Uuid;

use common::{create_box, create_rect_sketch, engine};

#[test]
fn undo_redo_roundtrip() {
    let mut eng = engine();
    let id = create_box(&mut eng);
    assert!(eng.can_undo());

    let events = eng.apply(Command::Undo).unwrap();
    assert!(events.contains(&Event::DocumentReset {
        reason: ResetReason::UndoRedo
    }));
    assert!(eng.part(id).is_none());
    assert!(eng.can_redo());

    eng.apply(Command::Redo).unwrap();
    assert!(eng.part(id).is_some());
}

#[test]
fn undo_on_empty_stack_is_noop() {
    let mut eng = engine();
    let events = eng.apply(Command::Undo).unwrap();
    assert!(events.is_empty());
}

#[test]
fn interactive_session_coalesces_to_one_undo_step() {
    let mut eng = engine();
    let id = create_box(&mut eng);
    let original = eng.part(id).unwrap().origin_transform;

    let session = Uuid::new_v4();
    for i in 1..=10 {
        eng.apply_interactive(
            session,
            Command::SetPartTransform {
                part_id: id,
                transform: Mat4::from_translation(Vec3::new(i as f32, 0.0, 0.0)),
            },
        )
        .unwrap();
    }
    eng.end_interaction(session, false).unwrap();

    // One undo restores the pre-drag transform, not an intermediate step
    eng.apply(Command::Undo).unwrap();
    assert_eq!(eng.part(id).unwrap().origin_transform, original);

    // The next undo removes the part creation itself
    eng.apply(Command::Undo).unwrap();
    assert!(eng.part(id).is_none());
}

#[test]
fn cancelled_interaction_rolls_back() {
    let mut eng = engine();
    let id = create_box(&mut eng);
    let original = eng.part(id).unwrap().origin_transform;

    let session = Uuid::new_v4();
    eng.apply_interactive(
        session,
        Command::SetPartTransform {
            part_id: id,
            transform: Mat4::from_translation(Vec3::X),
        },
    )
    .unwrap();
    let events = eng.end_interaction(session, true).unwrap();

    assert!(events.contains(&Event::DocumentReset {
        reason: ResetReason::UndoRedo
    }));
    assert_eq!(eng.part(id).unwrap().origin_transform, original);
    assert!(!eng.can_redo(), "cancel leaves no redo entry");
}

#[test]
fn set_joint_position_takes_no_snapshot() {
    let mut eng = engine();
    let parent = create_box(&mut eng);
    let child = create_box(&mut eng);
    let events = eng
        .apply(Command::ConnectParts {
            parent_part: parent,
            child_part: child,
        })
        .unwrap();
    let joint_id = events
        .iter()
        .find_map(|e| match e {
            Event::JointAdded { joint_id, .. } => Some(*joint_id),
            _ => None,
        })
        .unwrap();

    let desc_before = eng.undo_description().map(str::to_string);
    eng.apply(Command::SetJointPosition {
        joint_id,
        position: 0.1,
    })
    .unwrap();
    assert_eq!(
        eng.undo_description().map(str::to_string),
        desc_before,
        "joint slider does not pollute the undo stack"
    );
}

#[test]
fn failed_extrude_is_atomic() {
    let mut eng = engine();
    // A sketch with no entities has no closed profile, so the extrude
    // feature executes but creates no body -> error
    let sketch_id = Uuid::new_v4();
    eng.apply(Command::CreateSketch {
        id: Some(sketch_id),
        name: None,
        plane: rk_cad::SketchPlane::xy(),
    })
    .unwrap();
    let features_before = eng.features().len();
    let revision_before = eng.revision();

    let result = eng.apply(Command::AddExtrude {
        id: None,
        name: None,
        sketch_id,
        profiles: Vec::new(),
        distance: 5.0,
        direction: rk_cad::ExtrudeDirection::Positive,
        boolean_op: rk_cad::BooleanOp::New,
        target_body: None,
    });

    assert!(result.is_err());
    assert_eq!(eng.features().len(), features_before, "feature rolled back");
    assert!(eng.body_ids().is_empty());
    assert_eq!(eng.revision(), revision_before);
}

#[test]
fn undo_restores_cad_bodies() {
    let kernel = rk_cad::default_kernel();
    if !kernel.is_available() {
        return;
    }
    let mut eng = engine();
    let sketch_id = create_rect_sketch(&mut eng);

    eng.apply(Command::AddExtrude {
        id: None,
        name: None,
        sketch_id,
        profiles: Vec::new(),
        distance: 5.0,
        direction: rk_cad::ExtrudeDirection::Positive,
        boolean_op: rk_cad::BooleanOp::New,
        target_body: None,
    })
    .unwrap();
    let bodies = eng.body_ids();
    assert_eq!(bodies.len(), 1);

    // Undo removes the body, redo rebuilds it under the same ID
    eng.apply(Command::Undo).unwrap();
    assert!(eng.body_ids().is_empty());

    eng.apply(Command::Redo).unwrap();
    assert_eq!(eng.body_ids(), bodies);
}
