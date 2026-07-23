//! Document model: the complete editable state (parts/assembly + CAD data)
//! with versioned RON persistence.

use std::path::Path;

use serde::{Deserialize, Serialize};

use rk_cad::CadData;
use rk_core::{Assembly, MaterialDef, Part, Project};

/// Current document file format version.
///
/// - v1: `rk_core::Project` fields only (no `cad`)
/// - v2: v1 fields plus `cad` (sketches / feature history)
pub const DOCUMENT_VERSION: u32 = 2;

/// The complete editable state owned by the engine.
#[derive(Debug, Clone, Default)]
pub struct Document {
    pub project: Project,
    pub cad: CadData,
}

/// Serialized representation. Field names must match rk-core's private
/// `ProjectData` so that v1 project files parse as documents unchanged.
#[derive(Serialize, Deserialize)]
struct DocumentData {
    version: u32,
    name: String,
    parts: Vec<Part>,
    assembly: Assembly,
    materials: Vec<MaterialDef>,
    #[serde(default)]
    cad: CadData,
}

impl From<DocumentData> for Document {
    fn from(data: DocumentData) -> Self {
        let parts = data.parts.into_iter().map(|p| (p.id, p)).collect();
        Self {
            project: Project::with_parts(data.name, parts, data.assembly, data.materials),
            cad: data.cad,
        }
    }
}

impl Document {
    /// Create a new empty document
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            project: Project::new(name),
            cad: CadData::new(),
        }
    }

    /// Serialize to pretty RON at the current format version
    pub fn to_ron_bytes(&self) -> Result<Vec<u8>, DocumentError> {
        let data = DocumentData {
            version: DOCUMENT_VERSION,
            name: self.project.name.clone(),
            parts: self.project.parts().values().cloned().collect(),
            assembly: self.project.assembly.clone(),
            materials: self.project.materials.clone(),
            cad: self.cad.clone(),
        };
        let content = ron::ser::to_string_pretty(&data, ron::ser::PrettyConfig::default())
            .map_err(|e| DocumentError::Serialize(e.to_string()))?;
        Ok(content.into_bytes())
    }

    /// Parse a document from RON bytes (accepts v1 and v2 files)
    pub fn from_ron_bytes(bytes: &[u8]) -> Result<Self, DocumentError> {
        let content =
            std::str::from_utf8(bytes).map_err(|e| DocumentError::Deserialize(e.to_string()))?;
        match ron::from_str::<DocumentData>(content) {
            Ok(data) => {
                if data.version > DOCUMENT_VERSION {
                    return Err(DocumentError::UnsupportedVersion(data.version));
                }
                Ok(data.into())
            }
            Err(e) => {
                // If the full parse failed but the file declares a newer
                // version, report that instead of a raw parse error
                #[derive(Deserialize)]
                struct VersionProbe {
                    version: u32,
                }
                if let Ok(probe) = ron::from_str::<VersionProbe>(content)
                    && probe.version > DOCUMENT_VERSION
                {
                    return Err(DocumentError::UnsupportedVersion(probe.version));
                }
                Err(DocumentError::Deserialize(e.to_string()))
            }
        }
    }

    /// Save to a file
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), DocumentError> {
        let content = self.to_ron_bytes()?;
        std::fs::write(path.as_ref(), content).map_err(|e| DocumentError::Io(e.to_string()))?;
        Ok(())
    }

    /// Load from a file (accepts v1 and v2 files).
    ///
    /// Note: CAD bodies are not stored in the file; callers must run
    /// `cad.history.rebuild(kernel)` afterwards to restore geometry.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, DocumentError> {
        let content =
            std::fs::read(path.as_ref()).map_err(|e| DocumentError::Io(e.to_string()))?;
        Self::from_ron_bytes(&content)
    }
}

/// Document persistence errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum DocumentError {
    #[error("unsupported document version {0} (newest supported is {DOCUMENT_VERSION})")]
    UnsupportedVersion(u32),
    #[error("IO error: {0}")]
    Io(String),
    #[error("serialization error: {0}")]
    Serialize(String),
    #[error("deserialization error: {0}")]
    Deserialize(String),
}
