//! Command execution: resulting state and emitted events

mod common;

use approx::assert_relative_eq;
use glam::{Mat4, Vec3};
use rk_core::{GeometryType, JointType, Pose};
use rk_engine::{Command, EngineError, Event, PrimitiveSpec};
use uuid::Uuid;

use common::{create_box, engine, part_added_id};

#[test]
fn create_primitive_with_explicit_id() {
    let mut eng = engine();
    let id = Uuid::new_v4();
    let events = eng
        .apply(Command::CreatePrimitive {
            id: Some(id),
            primitive: PrimitiveSpec::Sphere { radius: 0.05 },
            name: Some("ball".into()),
        })
        .unwrap();

    assert_eq!(part_added_id(&events), id);
    let part = eng.part(id).unwrap();
    assert_eq!(part.name, "ball");
    assert!(!part.vertices.is_empty());
    assert!(eng.is_modified());
}

#[test]
fn delete_part_emits_removed() {
    let mut eng = engine();
    let id = create_box(&mut eng);
    let events = eng.apply(Command::DeletePart { part_id: id }).unwrap();
    assert!(events.contains(&Event::PartRemoved { part_id: id }));
    assert!(eng.part(id).is_none());
}

#[test]
fn delete_missing_part_fails_atomically() {
    let mut eng = engine();
    let id = create_box(&mut eng);
    let err = eng
        .apply(Command::DeletePart {
            part_id: Uuid::new_v4(),
        })
        .unwrap_err();
    assert!(matches!(err, EngineError::NotFound { .. }));
    assert!(eng.part(id).is_some(), "document unchanged after failure");
}

#[test]
fn set_part_transform_emits_world_transforms() {
    let mut eng = engine();
    let id = create_box(&mut eng);
    let transform = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
    let events = eng
        .apply(Command::SetPartTransform { part_id: id, transform })
        .unwrap();

    let transforms = events
        .iter()
        .find_map(|e| match e {
            Event::WorldTransformsChanged { transforms } => Some(transforms.clone()),
            _ => None,
        })
        .expect("WorldTransformsChanged event");
    let (_, m) = transforms.iter().find(|(pid, _)| *pid == id).unwrap();
    assert_eq!(*m, transform, "free part renders with its origin transform");
}

#[test]
fn connect_parts_builds_links_and_joint() {
    let mut eng = engine();
    let parent = create_box(&mut eng);
    let child = create_box(&mut eng);

    let events = eng
        .apply(Command::ConnectParts {
            parent_part: parent,
            child_part: child,
        })
        .unwrap();

    let link_adds = events
        .iter()
        .filter(|e| matches!(e, Event::LinkAdded { .. }))
        .count();
    assert_eq!(link_adds, 2);
    assert!(events.iter().any(|e| matches!(e, Event::JointAdded { .. })));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::WorldTransformsChanged { .. }))
    );

    let assembly = eng.assembly();
    assert_eq!(assembly.links.len(), 2);
    assert_eq!(assembly.joints.len(), 1);
}

#[test]
fn connect_missing_part_fails() {
    let mut eng = engine();
    let parent = create_box(&mut eng);
    let err = eng
        .apply(Command::ConnectParts {
            parent_part: parent,
            child_part: Uuid::new_v4(),
        })
        .unwrap_err();
    assert!(matches!(err, EngineError::NotFound { .. }));
    assert!(eng.assembly().links.is_empty(), "rolled back");
}

#[test]
fn joint_position_clamps_to_limits() {
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

    // Revolute gets default limits
    eng.apply(Command::SetJointType {
        joint_id,
        joint_type: JointType::Revolute,
    })
    .unwrap();
    let limits = eng.assembly().joints[&joint_id].limits.unwrap();

    let events = eng
        .apply(Command::SetJointPosition {
            joint_id,
            position: limits.upper + 100.0,
        })
        .unwrap();
    let clamped = events
        .iter()
        .find_map(|e| match e {
            Event::JointPositionChanged { position, .. } => Some(*position),
            _ => None,
        })
        .unwrap();
    assert_eq!(clamped, limits.upper);
}

#[test]
fn set_joint_origin_keeps_child_world_pose() {
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

    let world_of = |eng: &rk_engine::Engine, part: Uuid| -> Mat4 {
        eng.part_render_transforms()
            .into_iter()
            .find(|(pid, _)| *pid == part)
            .map(|(_, m)| m)
            .unwrap()
    };
    let before = world_of(&eng, child);

    let origin = Pose::from_position([0.5, 0.0, 0.25]);
    eng.apply(Command::SetJointOrigin {
        joint_id,
        origin,
        keep_child_world_pose: true,
    })
    .unwrap();

    let after = world_of(&eng, child);
    for (a, b) in before.to_cols_array().iter().zip(after.to_cols_array()) {
        assert_relative_eq!(*a, b, epsilon = 1e-4);
    }
    assert_eq!(eng.assembly().joints[&joint_id].origin, origin);
}

#[test]
fn collision_lifecycle() {
    let mut eng = engine();
    let parent = create_box(&mut eng);
    let child = create_box(&mut eng);
    let events = eng
        .apply(Command::ConnectParts {
            parent_part: parent,
            child_part: child,
        })
        .unwrap();
    let link_id = events
        .iter()
        .find_map(|e| match e {
            Event::LinkAdded { link_id, .. } => Some(*link_id),
            _ => None,
        })
        .unwrap();

    // Links created via from_part start with one default collision
    let base_count = eng.assembly().get_link(link_id).unwrap().collisions.len();

    let events = eng
        .apply(Command::AddCollision {
            link_id,
            geometry: GeometryType::Box {
                size: [0.1, 0.1, 0.1],
            },
            origin: Pose::default(),
        })
        .unwrap();
    let index = events
        .iter()
        .find_map(|e| match e {
            Event::CollisionAdded { index, .. } => Some(*index),
            _ => None,
        })
        .expect("CollisionAdded event");
    assert_eq!(index, base_count);

    eng.apply(Command::SetCollisionOrigin {
        link_id,
        index,
        origin: Pose::from_position([1.0, 0.0, 0.0]),
    })
    .unwrap();

    let err = eng
        .apply(Command::RemoveCollision {
            link_id,
            index: 99,
        })
        .unwrap_err();
    assert!(matches!(err, EngineError::InvalidCommand(_)));

    eng.apply(Command::RemoveCollision { link_id, index })
        .unwrap();
    assert_eq!(
        eng.assembly().get_link(link_id).unwrap().collisions.len(),
        base_count
    );
}

#[test]
fn revision_increments_on_success_only() {
    let mut eng = engine();
    let r0 = eng.revision();
    create_box(&mut eng);
    assert_eq!(eng.revision(), r0 + 1);
    let _ = eng.apply(Command::DeletePart {
        part_id: Uuid::new_v4(),
    });
    assert_eq!(eng.revision(), r0 + 1, "failed command does not bump revision");
    assert_eq!(eng.command_log().len(), 1);
}
