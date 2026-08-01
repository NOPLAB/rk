//! CAD Kernel Abstraction Layer
//!
//! Provides a trait-based abstraction over different geometry kernels
//! (OpenCASCADE, Truck, etc.) to allow switching implementations.

mod traits;

#[cfg(feature = "opencascade")]
mod opencascade;
#[cfg(feature = "truck")]
mod truck;

pub use traits::*;

#[cfg(feature = "opencascade")]
pub use opencascade::OpenCascadeKernel;
#[cfg(feature = "truck")]
pub use truck::TruckKernel;
