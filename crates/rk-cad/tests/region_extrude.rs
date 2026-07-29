//! Sweeping a region that has a hole in it.
//!
//! Truck cannot subtract solids, so a washer can only exist if the face the
//! sweep starts from carries the hole as a second boundary. This pins that
//! down: it is the one thing the whole clickable-profile flow rests on.

#![cfg(feature = "truck")]

use glam::{Vec2, Vec3};
use rk_cad::kernel::{Axis3D, CadKernel, Region2D, TruckKernel, Wire2D};

fn ring(radius: f32, segments: u32, clockwise: bool) -> Wire2D {
    let mut points: Vec<Vec2> = (0..segments)
        .map(|i| {
            let a = (i as f32 / segments as f32) * std::f32::consts::TAU;
            Vec2::new(radius * a.cos(), radius * a.sin())
        })
        .collect();
    if clockwise {
        points.reverse();
    }
    Wire2D::new(points, true)
}

/// Volume from the divergence theorem over the tessellation
fn volume(kernel: &TruckKernel, solid: &rk_cad::kernel::Solid) -> f32 {
    let mesh = kernel.tessellate(solid, 0.01).expect("tessellate");
    let at = |i: u32| Vec3::from_array(mesh.vertices[i as usize]);
    let mut total = 0.0f32;
    for tri in mesh.indices.chunks(3) {
        let [i, j, k] = tri else { continue };
        total += at(*i).dot(at(*j).cross(at(*k))) / 6.0;
    }
    total.abs()
}

#[test]
fn a_hole_survives_the_extrusion() {
    let kernel = TruckKernel::new();
    assert!(kernel.supports_holes());

    let washer = Region2D {
        outer: ring(1.0, 48, false),
        holes: vec![ring(0.5, 48, true)],
    };
    let solid = kernel
        .extrude_region(&washer, Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z, 2.0)
        .expect("extruding a washer");

    // π(1² − 0.5²) × 2 ≈ 4.71, against 6.28 for the disc without its hole
    let got = volume(&kernel, &solid);
    let want = std::f32::consts::PI * (1.0 - 0.25) * 2.0;
    assert!(
        (got - want).abs() / want < 0.05,
        "washer volume {got} is not {want} — the hole was dropped",
    );
}

#[test]
fn a_hole_survives_the_revolution() {
    let kernel = TruckKernel::new();

    // A square well clear of the axis, with a square hole inside it: revolved
    // a full turn it makes a torus of rectangular section, hollowed out
    let outer = Wire2D::new(
        vec![
            Vec2::new(2.0, -1.0),
            Vec2::new(4.0, -1.0),
            Vec2::new(4.0, 1.0),
            Vec2::new(2.0, 1.0),
        ],
        true,
    );
    let hole = Wire2D::new(
        vec![
            Vec2::new(2.5, -0.5),
            Vec2::new(2.5, 0.5),
            Vec2::new(3.5, 0.5),
            Vec2::new(3.5, -0.5),
        ],
        true,
    );
    let region = Region2D {
        outer,
        holes: vec![hole],
    };

    let solid = kernel
        .revolve_region(
            &region,
            Vec3::ZERO,
            Vec3::X,
            Vec3::Z,
            &Axis3D::new(Vec3::ZERO, Vec3::Z),
            std::f32::consts::TAU,
        )
        .expect("revolving a hollow section");

    // Pappus: 2π × centroid radius × area, for the section minus the hole
    let want = std::f32::consts::TAU * (3.0 * 4.0 - 3.0 * 1.0);
    let got = volume(&kernel, &solid);
    assert!(
        (got - want).abs() / want < 0.05,
        "revolved volume {got} is not {want} — the hole was dropped",
    );
}
