//! Shared test helpers
// Each test binary only uses a subset of these
#![allow(dead_code)]

use std::sync::Arc;

use glam::Vec2;
use rk_engine::{Command, Engine, Event, PrimitiveSpec};
use uuid::Uuid;

pub fn engine() -> Engine {
    Engine::new(Arc::from(rk_cad::default_kernel()))
}

pub fn create_box(engine: &mut Engine) -> Uuid {
    let events = engine
        .apply(Command::CreatePrimitive {
            id: None,
            primitive: PrimitiveSpec::Box {
                size: [0.1, 0.1, 0.1],
            },
            name: None,
        })
        .unwrap();
    part_added_id(&events)
}

pub fn part_added_id(events: &[Event]) -> Uuid {
    events
        .iter()
        .find_map(|e| match e {
            Event::PartAdded { part_id } => Some(*part_id),
            _ => None,
        })
        .expect("PartAdded event")
}

/// Create a sketch with a closed rectangle profile, returning its ID
pub fn create_rect_sketch(engine: &mut Engine) -> Uuid {
    rect_sketch(engine, Vec2::ZERO, Vec2::new(10.0, 10.0))
}

/// Create a sketch on XY holding one closed rectangle, returning its ID
pub fn rect_sketch(engine: &mut Engine, min: Vec2, max: Vec2) -> Uuid {
    let sketch_id = Uuid::new_v4();
    engine
        .apply(Command::CreateSketch {
            id: Some(sketch_id),
            name: None,
            plane: rk_cad::SketchPlane::xy(),
        })
        .unwrap();

    let mut sketch = rk_cad::Sketch::new("tmp", rk_cad::SketchPlane::xy());
    sketch.add_rectangle(min, max);
    let entities: Vec<_> = sketch.entities().values().cloned().collect();
    engine
        .apply(Command::AddSketchEntities {
            sketch_id,
            entities,
        })
        .unwrap();
    sketch_id
}

/// Volume enclosed by a body's display mesh, by the divergence theorem
pub fn body_volume(engine: &mut Engine, body: Uuid) -> f32 {
    let mesh = engine.body_mesh(body).unwrap();
    mesh.indices
        .chunks(3)
        .filter_map(|tri| match tri {
            [i, j, k] => {
                let at = |n: &u32| glam::Vec3::from_array(mesh.vertices[*n as usize]);
                Some(at(i).dot(at(j).cross(at(k))) / 6.0)
            }
            _ => None,
        })
        .sum::<f32>()
        .abs()
}
