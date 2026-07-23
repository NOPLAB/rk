//! Protocol compatibility: every Command and Event variant must survive
//! a JSON roundtrip unchanged (commands/events travel as JSON over the
//! future MCP / JSON-RPC boundary; RON is only used for document files
//! and does not support internally tagged enums). The exhaustive match
//! in `assert_command_coverage` forces this file to be updated whenever
//! a variant is added.

use std::path::PathBuf;

use glam::{Mat4, Vec2, Vec3};
use rk_cad::{
    BooleanOp, ExtrudeDirection, SketchConstraint, SketchPlane, Wire2D,
};
use rk_core::{GeometryType, InertiaMatrix, JointType, Pose, StlUnit};
use rk_engine::{Command, Event, ExtrudePreviewRequest, PrimitiveSpec, ResetReason};
use uuid::Uuid;

fn sample_entities() -> Vec<rk_cad::SketchEntity> {
    let mut sketch = rk_cad::Sketch::new("s", SketchPlane::xy());
    let a = sketch.add_point(Vec2::ZERO);
    let b = sketch.add_point(Vec2::new(1.0, 0.0));
    sketch.add_line(a, b);
    sketch.entities().values().cloned().collect()
}

fn all_commands() -> Vec<Command> {
    let id = Uuid::new_v4;
    let entities = sample_entities();
    let entity_id = entities[0].id();

    vec![
        Command::NewDocument,
        Command::LoadDocument {
            path: PathBuf::from("a.rk"),
        },
        Command::SaveDocument {
            path: Some(PathBuf::from("a.rk")),
        },
        Command::ImportMesh {
            path: PathBuf::from("m.stl"),
            unit: StlUnit::default(),
        },
        Command::ImportUrdf {
            path: PathBuf::from("r.urdf"),
            stl_unit: StlUnit::default(),
        },
        Command::ExportUrdf {
            path: PathBuf::from("out"),
            robot_name: "robot".into(),
        },
        Command::CreatePrimitive {
            id: Some(id()),
            primitive: PrimitiveSpec::Box {
                size: [0.1, 0.2, 0.3],
            },
            name: Some("box".into()),
        },
        Command::CreatePrimitive {
            id: None,
            primitive: PrimitiveSpec::Cylinder {
                radius: 0.05,
                height: 0.1,
            },
            name: None,
        },
        Command::CreatePrimitive {
            id: None,
            primitive: PrimitiveSpec::Sphere { radius: 0.05 },
            name: None,
        },
        Command::CreateEmptyPart {
            id: Some(id()),
            name: None,
        },
        Command::DeletePart { part_id: id() },
        Command::RenamePart {
            part_id: id(),
            name: "renamed".into(),
        },
        Command::SetPartTransform {
            part_id: id(),
            transform: Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0)),
        },
        Command::SetPartColor {
            part_id: id(),
            color: [0.1, 0.2, 0.3, 1.0],
        },
        Command::SetPartMass {
            part_id: id(),
            mass: 2.5,
        },
        Command::SetPartInertia {
            part_id: id(),
            inertia: InertiaMatrix::default(),
        },
        Command::ConnectParts {
            parent_part: id(),
            child_part: id(),
        },
        Command::DisconnectPart { child_part: id() },
        Command::SetJointPosition {
            joint_id: id(),
            position: 0.5,
        },
        Command::ResetJointPosition { joint_id: id() },
        Command::ResetAllJointPositions,
        Command::SetJointType {
            joint_id: id(),
            joint_type: JointType::Revolute,
        },
        Command::SetJointOrigin {
            joint_id: id(),
            origin: Pose::from_position([1.0, 0.0, 0.0]),
            keep_child_world_pose: true,
        },
        Command::SetJointAxis {
            joint_id: id(),
            axis: Vec3::Z,
        },
        Command::SetJointLimits {
            joint_id: id(),
            limits: None,
        },
        Command::AddCollision {
            link_id: id(),
            geometry: GeometryType::Capsule {
                radius: 0.05,
                length: 0.2,
            },
            origin: Pose::default(),
        },
        Command::RemoveCollision {
            link_id: id(),
            index: 1,
        },
        Command::SetCollisionOrigin {
            link_id: id(),
            index: 0,
            origin: Pose::default(),
        },
        Command::SetCollisionGeometry {
            link_id: id(),
            index: 0,
            geometry: GeometryType::Sphere { radius: 0.1 },
        },
        Command::CreateSketch {
            id: Some(id()),
            name: Some("sketch".into()),
            plane: SketchPlane::xz(),
        },
        Command::DeleteSketch { sketch_id: id() },
        Command::AddSketchEntities {
            sketch_id: id(),
            entities: entities.clone(),
        },
        Command::UpdateSketchEntity {
            sketch_id: id(),
            entity: entities[0].clone(),
        },
        Command::DeleteSketchEntities {
            sketch_id: id(),
            entity_ids: vec![entity_id],
        },
        Command::AddSketchConstraint {
            sketch_id: id(),
            constraint: SketchConstraint::distance(id(), id(), 5.0),
        },
        Command::DeleteSketchConstraint {
            sketch_id: id(),
            constraint_id: id(),
        },
        Command::SolveSketch { sketch_id: id() },
        Command::AddExtrude {
            id: Some(id()),
            name: None,
            sketch_id: id(),
            distance: 5.0,
            direction: ExtrudeDirection::Symmetric,
            boolean_op: BooleanOp::Join,
            target_body: Some(id()),
        },
        Command::AddRevolve {
            id: None,
            name: Some("rev".into()),
            sketch_id: id(),
            axis_origin: Vec3::ZERO,
            axis_direction: Vec3::Y,
            angle: std::f32::consts::PI,
            boolean_op: BooleanOp::New,
            target_body: None,
        },
        Command::DeleteFeature { feature_id: id() },
        Command::SetFeatureSuppressed {
            feature_id: id(),
            suppressed: true,
        },
        Command::RollbackTo {
            feature_id: Some(id()),
        },
        Command::RebuildFeatures,
        Command::Undo,
        Command::Redo,
    ]
}

fn all_events() -> Vec<Event> {
    let id = Uuid::new_v4;
    vec![
        Event::DocumentReset {
            reason: ResetReason::Loaded,
        },
        Event::DocumentSaved {
            path: PathBuf::from("a.rk"),
        },
        Event::ModifiedChanged { modified: true },
        Event::PartAdded { part_id: id() },
        Event::PartRemoved { part_id: id() },
        Event::PartRenamed {
            part_id: id(),
            name: "n".into(),
        },
        Event::PartAppearanceChanged { part_id: id() },
        Event::PartPhysicsChanged { part_id: id() },
        Event::WorldTransformsChanged {
            transforms: vec![(id(), Mat4::IDENTITY), (id(), Mat4::from_translation(Vec3::X))],
        },
        Event::LinkAdded {
            link_id: id(),
            part_id: Some(id()),
        },
        Event::JointAdded {
            joint_id: id(),
            parent_link: id(),
            child_link: id(),
        },
        Event::JointRemoved { joint_id: id() },
        Event::JointChanged { joint_id: id() },
        Event::JointPositionChanged {
            joint_id: id(),
            position: 0.3,
        },
        Event::CollisionAdded {
            link_id: id(),
            index: 0,
        },
        Event::CollisionRemoved {
            link_id: id(),
            index: 0,
        },
        Event::CollisionChanged {
            link_id: id(),
            index: 0,
        },
        Event::SketchAdded { sketch_id: id() },
        Event::SketchRemoved { sketch_id: id() },
        Event::SketchGeometryChanged { sketch_id: id() },
        Event::SketchSolved { sketch_id: id() },
        Event::FeatureAdded { feature_id: id() },
        Event::FeatureRemoved { feature_id: id() },
        Event::FeatureChanged { feature_id: id() },
        Event::BodiesRebuilt {
            body_ids: vec![id(), id()],
        },
        Event::HistoryChanged {
            can_undo: true,
            can_redo: false,
            undo_description: Some("Move Part".into()),
        },
    ]
}

/// Compile-time exhaustiveness: fails to build when a variant is added
/// without updating the sample lists above.
#[allow(dead_code)]
fn assert_command_coverage(cmd: &Command) {
    use Command::*;
    match cmd {
        NewDocument | LoadDocument { .. } | SaveDocument { .. } | ImportMesh { .. }
        | ImportUrdf { .. } | ExportUrdf { .. } | CreatePrimitive { .. }
        | CreateEmptyPart { .. } | DeletePart { .. } | RenamePart { .. }
        | SetPartTransform { .. } | SetPartColor { .. } | SetPartMass { .. }
        | SetPartInertia { .. } | ConnectParts { .. } | DisconnectPart { .. }
        | SetJointPosition { .. } | ResetJointPosition { .. } | ResetAllJointPositions
        | SetJointType { .. } | SetJointOrigin { .. } | SetJointAxis { .. }
        | SetJointLimits { .. } | AddCollision { .. } | RemoveCollision { .. }
        | SetCollisionOrigin { .. } | SetCollisionGeometry { .. } | CreateSketch { .. }
        | DeleteSketch { .. } | AddSketchEntities { .. } | UpdateSketchEntity { .. }
        | DeleteSketchEntities { .. } | AddSketchConstraint { .. }
        | DeleteSketchConstraint { .. } | SolveSketch { .. } | AddExtrude { .. }
        | AddRevolve { .. } | DeleteFeature { .. } | SetFeatureSuppressed { .. }
        | RollbackTo { .. } | RebuildFeatures | Undo | Redo => {}
    }
}

#[test]
fn commands_roundtrip_json() {
    for cmd in all_commands() {
        let json = serde_json::to_string(&cmd).unwrap();
        let back: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, back, "JSON roundtrip failed for {json}");
    }
}

#[test]
fn events_roundtrip_json() {
    for event in all_events() {
        let json = serde_json::to_string(&event).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(event, back, "JSON roundtrip failed for {json}");
    }
}

#[test]
fn preview_request_roundtrips() {
    let req = ExtrudePreviewRequest {
        sketch_id: Uuid::new_v4(),
        profiles: vec![Wire2D::rectangle(Vec2::ZERO, 2.0, 1.0)],
        distance: 3.0,
        direction: ExtrudeDirection::Negative,
    };
    let json = serde_json::to_string(&req).unwrap();
    let back: ExtrudePreviewRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(req, back);
}
