//! URDF Editor Core Data Structures
//!
//! This crate contains the core data structures for URDF editing:
//! - Part: STL mesh with metadata
//! - JointPoint: Connection points on parts
//! - Assembly: Scene graph for robot structure
//! - Project: Serializable project file

pub mod assembly;
pub mod export;
pub mod import;
pub mod inertia;
pub mod mesh;
pub mod part;
pub mod primitive;
pub mod project;
pub mod stl;

pub use assembly::*;
pub use export::*;
pub use import::*;
pub use inertia::*;
pub use mesh::*;
pub use part::*;
pub use primitive::*;
pub use project::*;
pub use stl::*;
