//! The three primitives, from whichever kernel is compiled in.
//!
//! Nothing covered these before, which is how a sphere came to be built as a
//! half sphere and stayed that way.

#![cfg(any(feature = "truck", feature = "opencascade"))]

use glam::Vec3;
use rk_cad::kernel::CadKernel;

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

fn check(who: &str, what: &str, got: f32, want: f32) {
    assert!(
        (got - want).abs() / want < 0.05,
        "{who}: {what} came out at {got}, not {want}",
    );
}

#[test]
fn a_box_is_its_own_size() {
    for kernel in kernels() {
        let solid = kernel
            .create_box(Vec3::new(1.0, 2.0, 3.0), Vec3::new(10.0, 10.0, 10.0))
            .expect("a 10-cube");
        check(
            kernel.name(),
            "box volume",
            volume(kernel.as_ref(), &solid),
            1000.0,
        );
    }
}

#[test]
fn a_cylinder_is_round() {
    for kernel in kernels() {
        let solid = kernel
            .create_cylinder(Vec3::ZERO, 2.0, 5.0, Vec3::Z)
            .expect("a cylinder");
        check(
            kernel.name(),
            "cylinder volume",
            volume(kernel.as_ref(), &solid),
            std::f32::consts::PI * 4.0 * 5.0,
        );
    }
}

/// A sphere swept half a turn is a hemisphere, and half the volume is the
/// only thing that says so
#[test]
fn a_sphere_is_whole() {
    for kernel in kernels() {
        let solid = kernel.create_sphere(Vec3::ZERO, 3.0).expect("a sphere");
        check(
            kernel.name(),
            "sphere volume",
            volume(kernel.as_ref(), &solid),
            4.0 / 3.0 * std::f32::consts::PI * 27.0,
        );
    }
}
