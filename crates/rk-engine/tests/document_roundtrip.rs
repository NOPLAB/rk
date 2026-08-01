//! Document persistence: v1 compatibility, v2 roundtrip, version guard,
//! and geometry rebuild after load.

use glam::Vec2;
use rk_cad::{ExtrudeDirection, Feature, Sketch, SketchPlane, default_kernel};
use rk_core::{Part, Project};
use rk_engine::{Document, DocumentError};
use uuid::Uuid;

/// Pins the top-level v1 field names. If this test breaks, a rename in
/// rk-core's ProjectData (or DocumentData drifting from it) has broken
/// compatibility with existing .rk files.
#[test]
fn v1_fixture_loads_with_empty_cad() {
    let fixture = r#"(
    version: 1,
    name: "legacy project",
    parts: [],
    assembly: (
        name: "robot",
        links: {},
        joints: {},
        children: {},
        parent: {},
    ),
    materials: [],
)"#;

    let doc = Document::from_ron_bytes(fixture.as_bytes()).unwrap();
    assert_eq!(doc.project.name, "legacy project");
    assert_eq!(doc.project.assembly.name, "robot");
    assert!(doc.project.parts().is_empty());
    assert!(doc.cad.is_empty());
}

/// A file written by rk-core's v1 Project::save must load as a Document.
#[test]
fn v1_file_written_by_rk_core_loads() {
    let mut project = Project::new("from rk-core");
    let mut part = Part::new("tri");
    part.vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    part.normals = vec![[0.0, 0.0, 1.0]; 3];
    part.indices = vec![0, 1, 2];
    part.color = [0.2, 0.4, 0.6, 1.0];
    let part_id = project.add_part(part);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.rk");
    project.save(&path).unwrap();

    let doc = Document::load(&path).unwrap();
    assert_eq!(doc.project.name, "from rk-core");
    assert!(doc.cad.is_empty());
    let loaded = doc.project.get_part(part_id).expect("part survives");
    assert_eq!(loaded.name, "tri");
    assert_eq!(loaded.vertices.len(), 3);
    assert_eq!(loaded.color, [0.2, 0.4, 0.6, 1.0]);
}

#[test]
fn v2_roundtrip_preserves_parts_and_cad() {
    let mut doc = Document::new("v2 doc");

    let mut part = Part::new("p1");
    part.vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    part.indices = vec![0, 1, 2];
    let part_id = doc.project.add_part(part);

    let mut sketch = Sketch::new("profile", SketchPlane::xy());
    sketch.add_rectangle(Vec2::ZERO, Vec2::new(10.0, 10.0));
    let entity_count = sketch.entities().len();
    let sketch_id = doc.cad.history.add_sketch(sketch);
    let feature = Feature::extrude("E1", sketch_id, 5.0, ExtrudeDirection::Positive);
    let feature_id = feature.id();
    doc.cad.history.add_feature(feature);

    let bytes = doc.to_ron_bytes().unwrap();
    let loaded = Document::from_ron_bytes(&bytes).unwrap();

    assert_eq!(loaded.project.name, "v2 doc");
    assert!(loaded.project.get_part(part_id).is_some());
    let sketch = loaded.cad.history.get_sketch(sketch_id).expect("sketch");
    assert_eq!(sketch.entities().len(), entity_count);
    assert!(loaded.cad.history.get_by_id(feature_id).is_some());
}

#[test]
fn newer_version_is_rejected() {
    let fixture = r#"(
    version: 99,
    name: "from the future",
    parts: [],
    assembly: (name: "robot", links: {}, joints: {}, children: {}, parent: {}),
    materials: [],
)"#;

    match Document::from_ron_bytes(fixture.as_bytes()) {
        Err(DocumentError::UnsupportedVersion(v)) => assert_eq!(v, 99),
        other => panic!("expected UnsupportedVersion, got {other:?}"),
    }
}

/// Bodies are not stored in the file; after load a rebuild must restore
/// them under the same IDs that were recorded before saving.
#[test]
fn rebuild_after_load_restores_bodies_with_stable_ids() {
    let kernel = default_kernel();
    if !kernel.is_available() {
        return; // NullKernel build; nothing to execute
    }

    let mut doc = Document::new("cad doc");
    let mut sketch = Sketch::new("profile", SketchPlane::xy());
    sketch.add_rectangle(Vec2::ZERO, Vec2::new(10.0, 10.0));
    let sketch_id = doc.cad.history.add_sketch(sketch);
    doc.cad.history.add_feature(Feature::extrude(
        "E1",
        sketch_id,
        5.0,
        ExtrudeDirection::Positive,
    ));

    doc.cad.history.rebuild(&*kernel).unwrap();
    let mut ids_before: Vec<Uuid> = doc.cad.history.bodies().keys().copied().collect();
    ids_before.sort();
    assert_eq!(ids_before.len(), 1);

    let bytes = doc.to_ron_bytes().unwrap();
    let mut loaded = Document::from_ron_bytes(&bytes).unwrap();
    assert!(
        loaded.cad.history.bodies().is_empty(),
        "bodies are serde-skipped"
    );

    loaded.cad.history.rebuild(&*kernel).unwrap();
    let mut ids_after: Vec<Uuid> = loaded.cad.history.bodies().keys().copied().collect();
    ids_after.sort();
    assert_eq!(ids_before, ids_after);
}
