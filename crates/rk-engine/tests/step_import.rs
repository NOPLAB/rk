//! STEP import, which exists at all only because the kernel is OpenCASCADE.
//!
//! The interesting part is units. STEP files carry their own, OpenCASCADE's
//! reader normalises whatever it finds to millimetres, and the scene is in
//! metres — so an import that forwards the numbers untouched puts a 100 mm
//! bracket 100 m across.

mod common;

use common::engine;
use glam::Vec3;
use rk_core::StlUnit;
use rk_engine::Command;

fn opencascade() -> bool {
    rk_cad::default_kernel().name() == "opencascade"
}

#[test]
fn a_step_file_in_millimetres_arrives_in_metres() {
    if !opencascade() {
        return; // no other kernel reads STEP
    }

    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("bracket.step");

    // Author the file with the kernel itself: a 100 x 200 x 300 box, written
    // under the millimetre unit OpenCASCADE declares by default. The numbers
    // in the file are therefore millimetres, exactly as a real CAD system
    // would write them.
    let kernel = rk_cad::default_kernel();
    let solid = kernel
        .create_box(Vec3::ZERO, Vec3::new(100.0, 200.0, 300.0))
        .expect("a box");
    kernel
        .export_step(&solid, &path, &Default::default())
        .expect("writing the STEP file");

    let mut eng = engine();
    let events = eng
        .apply(Command::ImportMesh {
            path,
            unit: StlUnit::Millimeters,
        })
        .expect("importing the STEP file");

    let part_id = common::part_added_id(&events);
    let part = eng.part(part_id).expect("the imported part");
    let size = Vec3::from(part.bbox_max) - Vec3::from(part.bbox_min);

    let want = Vec3::new(0.1, 0.2, 0.3);
    assert!(
        (size - want).length() < 1e-3,
        "a 100 x 200 x 300 mm box imported as {size}, not {want} metres",
    );
}
