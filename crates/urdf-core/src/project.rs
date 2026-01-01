//! Project file serialization

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::assembly::Assembly;
use crate::part::Part;

/// Project file containing all editor state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// File format version
    pub version: u32,
    /// Project name
    pub name: String,
    /// All parts in the project
    pub parts: Vec<Part>,
    /// Robot assembly
    pub assembly: Assembly,
    /// Material definitions
    pub materials: Vec<MaterialDef>,
}

impl Default for Project {
    fn default() -> Self {
        Self::new("New Project")
    }
}

impl Project {
    /// Create a new empty project
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            version: 1,
            name: name.into(),
            parts: Vec::new(),
            assembly: Assembly::default(),
            materials: Vec::new(),
        }
    }

    /// Save project to a file
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ProjectError> {
        let path = path.as_ref();
        let content = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|e| ProjectError::Serialize(e.to_string()))?;
        std::fs::write(path, content).map_err(|e| ProjectError::Io(e.to_string()))?;
        Ok(())
    }

    /// Load project from a file
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ProjectError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| ProjectError::Io(e.to_string()))?;
        let project: Project =
            ron::from_str(&content).map_err(|e| ProjectError::Deserialize(e.to_string()))?;
        Ok(project)
    }

    /// Add a part to the project
    pub fn add_part(&mut self, part: Part) {
        self.parts.push(part);
    }

    /// Get a part by ID
    pub fn get_part(&self, id: uuid::Uuid) -> Option<&Part> {
        self.parts.iter().find(|p| p.id == id)
    }

    /// Get a mutable part by ID
    pub fn get_part_mut(&mut self, id: uuid::Uuid) -> Option<&mut Part> {
        self.parts.iter_mut().find(|p| p.id == id)
    }

    /// Remove a part by ID
    pub fn remove_part(&mut self, id: uuid::Uuid) -> Option<Part> {
        if let Some(pos) = self.parts.iter().position(|p| p.id == id) {
            Some(self.parts.remove(pos))
        } else {
            None
        }
    }
}

/// Material definition for URDF
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDef {
    pub name: String,
    pub color: [f32; 4],
    pub texture: Option<String>,
}

impl MaterialDef {
    pub fn new(name: impl Into<String>, color: [f32; 4]) -> Self {
        Self {
            name: name.into(),
            color,
            texture: None,
        }
    }
}

/// Project-related errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ProjectError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Serialization error: {0}")]
    Serialize(String),
    #[error("Deserialization error: {0}")]
    Deserialize(String),
}
