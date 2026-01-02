//! URDF Editor Core Data Structures
//!
//! This crate contains the core data structures for URDF editing:
//! - Part: STL mesh with metadata
//! - JointPoint: Connection points on parts
//! - Assembly: Scene graph for robot structure
//! - Project: Serializable project file

pub mod part;
pub mod assembly;
pub mod inertia;
pub mod project;
pub mod export;
pub mod import;
pub mod primitive;
pub mod stl;

pub use part::*;
pub use assembly::*;
pub use inertia::*;
pub use project::*;
pub use export::*;
pub use import::*;
pub use primitive::*;
pub use stl::*;
