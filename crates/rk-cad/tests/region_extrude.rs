//! Sweeping a region that has a hole in it.
//!
//! A washer is the one thing the whole clickable-profile flow rests on, and
//! the two kernels reach it by different routes — truck attaches the island
//! as a second boundary of the face it sweeps, OpenCASCADE cuts the island's
//! own sweep back out. Whichever is compiled in has to get there.

// Nothing to prove in a build configured without a kernel, and a suite that
// reports three green tests without touching any geometry is worse than one
// that reports none
#![cfg(any(feature = "truck", feature = "opencascade"))]

use glam::{Vec2, Vec3};
use rk_cad::kernel::{Axis3D, CadKernel, Region2D, Wire2D};

/// Every kernel this build actually has
fn kernels() -> Vec<Box<dyn CadKernel>> {
    let makers: &[fn() -> Box<dyn CadKernel>] = &[
        #[cfg(feature = "truck")]
        (|| Box::new(rk_cad::kernel::TruckKernel::new())),
        #[cfg(feature = "opencascade")]
        (|| Box::new(rk_cad::kernel::OpenCascadeKernel::new())),
    ];
    makers.iter().map(|make| make()).collect()
}

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
fn volume(kernel: &dyn CadKernel, solid: &rk_cad::kernel::Solid) -> f32 {
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
    for kernel in kernels() {
        let who = kernel.name();
        assert!(kernel.supports_holes(), "{who} does not carry islands");

        let washer = Region2D {
            outer: ring(1.0, 48, false),
            holes: vec![ring(0.5, 48, true)],
        };
        let solid = kernel
            .extrude_region(&washer, Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z, 2.0)
            .expect("extruding a washer");

        // π(1² − 0.5²) × 2 ≈ 4.71, against 6.28 for the disc without its hole
        let got = volume(kernel.as_ref(), &solid);
        let want = std::f32::consts::PI * (1.0 - 0.25) * 2.0;
        assert!(
            (got - want).abs() / want < 0.05,
            "{who}: washer volume {got} is not {want} — the hole was dropped",
        );
    }
}

#[test]
fn a_hole_survives_the_revolution() {
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

    for kernel in kernels() {
        let who = kernel.name();
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
        let got = volume(kernel.as_ref(), &solid);
        assert!(
            (got - want).abs() / want < 0.05,
            "{who}: revolved volume {got} is not {want} — the hole was dropped",
        );
    }
}

/// A partial revolution leaves the island's end caps sitting exactly in the
/// body's own — the case where cutting the island out could plausibly go wrong
#[test]
fn a_hole_survives_a_quarter_revolution() {
    let region = Region2D {
        outer: Wire2D::new(
            vec![
                Vec2::new(2.0, -1.0),
                Vec2::new(4.0, -1.0),
                Vec2::new(4.0, 1.0),
                Vec2::new(2.0, 1.0),
            ],
            true,
        ),
        holes: vec![Wire2D::new(
            vec![
                Vec2::new(2.5, -0.5),
                Vec2::new(2.5, 0.5),
                Vec2::new(3.5, 0.5),
                Vec2::new(3.5, -0.5),
            ],
            true,
        )],
    };

    for kernel in kernels() {
        let who = kernel.name();
        let solid = kernel
            .revolve_region(
                &region,
                Vec3::ZERO,
                Vec3::X,
                Vec3::Z,
                &Axis3D::new(Vec3::ZERO, Vec3::Z),
                std::f32::consts::FRAC_PI_2,
            )
            .expect("revolving a hollow section a quarter turn");

        let want = std::f32::consts::TAU * (3.0 * 4.0 - 3.0 * 1.0) / 4.0;
        let got = volume(kernel.as_ref(), &solid);
        assert!(
            (got - want).abs() / want < 0.05,
            "{who}: quarter-revolved volume {got} is not {want}",
        );
    }
}
