//! Document lifecycle and file I/O commands

use std::collections::HashMap;
use std::path::PathBuf;

use rk_core::{ExportOptions, ImportOptions, StlUnit, export_urdf, import_urdf, load_mesh_multi};

use crate::document::Document;
use crate::engine::Engine;
use crate::error::EngineError;
use crate::event::{Event, ResetReason};

impl Engine {
    pub(crate) fn exec_new_document(&mut self, events: &mut Vec<Event>) -> Result<(), EngineError> {
        self.doc = Document::default();
        self.doc_path = None;
        self.undo.clear();
        events.push(Event::DocumentReset {
            reason: ResetReason::New,
        });
        events.push(self.history_changed_event());
        Ok(())
    }

    pub(crate) fn exec_load_document(
        &mut self,
        path: PathBuf,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let doc = Document::load(&path)?;
        self.restore_snapshot(doc, ResetReason::Loaded, events);
        self.doc_path = Some(path);
        self.undo.clear();
        events.push(self.history_changed_event());
        Ok(())
    }

    pub(crate) fn exec_save_document(
        &mut self,
        path: Option<PathBuf>,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let path = path
            .or_else(|| self.doc_path.clone())
            .ok_or_else(|| EngineError::InvalidCommand("no save path set".into()))?;
        self.doc.save(&path)?;
        tracing::info!("saved document to {:?}", path);
        self.doc_path = Some(path.clone());
        events.push(Event::DocumentSaved { path });
        Ok(())
    }

    pub(crate) fn exec_import_mesh(
        &mut self,
        path: PathBuf,
        unit: StlUnit,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        // A STEP assembly arrives as one part per solid, and dropping all
        // but the first is not an import. Every other format returns one.
        let parts = load_mesh_multi(&path, unit).map_err(|e| EngineError::Io(e.to_string()))?;
        for part in parts {
            tracing::info!(
                "imported mesh: {} ({} vertices, unit={:?})",
                part.name,
                part.vertices.len(),
                unit
            );
            let part_id = self.doc.project.add_part(part);
            events.push(Event::PartAdded { part_id });
        }
        Ok(())
    }

    pub(crate) fn exec_import_urdf(
        &mut self,
        path: PathBuf,
        stl_unit: StlUnit,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let options = ImportOptions {
            base_dir: path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from(".")),
            stl_unit,
            default_color: [0.7, 0.7, 0.7, 1.0],
            package_paths: HashMap::new(),
        };
        let project = import_urdf(&path, &options).map_err(|e| EngineError::Io(e.to_string()))?;
        tracing::info!(
            "imported URDF: {} ({} links, {} joints, {} parts)",
            project.name,
            project.assembly.links.len(),
            project.assembly.joints.len(),
            project.parts().len()
        );
        // The import replaces the whole document. Unlike loading a project
        // file, the URDF path is not a document save path.
        self.restore_snapshot(
            Document {
                project,
                cad: Default::default(),
            },
            ResetReason::UrdfImported,
            events,
        );
        self.doc_path = None;
        Ok(())
    }

    pub(crate) fn exec_export_urdf(
        &mut self,
        path: PathBuf,
        robot_name: String,
    ) -> Result<(), EngineError> {
        let options = ExportOptions {
            output_dir: path,
            robot_name,
            mesh_prefix: "meshes".to_string(),
            use_package_uri: false,
        };
        export_urdf(
            &self.doc.project.assembly,
            self.doc.project.parts(),
            &options,
        )
        .map_err(|e| EngineError::Io(e.to_string()))?;
        tracing::info!("exported URDF to {:?}", options.output_dir);
        Ok(())
    }
}
