//! Feature commands (extrude, revolve, suppression, rollback) and the
//! extrude preview query.

use glam::Vec3;
use uuid::Uuid;

use rk_cad::{
    BooleanOp, BooleanType, CadKernel, ExtrudeDirection, Feature, Sketch, Solid, TessellatedMesh,
    Wire2D,
};

use crate::command::ExtrudePreviewRequest;
use crate::engine::{DISPLAY_MESH_TOLERANCE, Engine};
use crate::error::EngineError;
use crate::event::Event;

impl Engine {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn exec_add_extrude(
        &mut self,
        id: Option<Uuid>,
        name: Option<String>,
        sketch_id: Uuid,
        distance: f32,
        direction: ExtrudeDirection,
        boolean_op: BooleanOp,
        target_body: Option<Uuid>,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        if self.doc.cad.history.get_sketch(sketch_id).is_none() {
            return Err(EngineError::NotFound {
                kind: "sketch",
                id: sketch_id,
            });
        }
        let feature = Feature::Extrude {
            id: id.unwrap_or_else(Uuid::new_v4),
            name: name.unwrap_or_else(|| "Extrude".to_string()),
            sketch_id,
            distance,
            direction,
            boolean_op,
            target_body,
            draft_angle: 0.0,
            suppressed: false,
        };
        self.add_feature_and_rebuild(feature, boolean_op, events)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn exec_add_revolve(
        &mut self,
        id: Option<Uuid>,
        name: Option<String>,
        sketch_id: Uuid,
        axis_origin: Vec3,
        axis_direction: Vec3,
        angle: f32,
        boolean_op: BooleanOp,
        target_body: Option<Uuid>,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        if self.doc.cad.history.get_sketch(sketch_id).is_none() {
            return Err(EngineError::NotFound {
                kind: "sketch",
                id: sketch_id,
            });
        }
        let feature = Feature::Revolve {
            id: id.unwrap_or_else(Uuid::new_v4),
            name: name.unwrap_or_else(|| "Revolve".to_string()),
            sketch_id,
            axis_origin,
            axis_direction,
            angle,
            boolean_op,
            target_body,
            suppressed: false,
        };
        self.add_feature_and_rebuild(feature, boolean_op, events)
    }

    /// Add a feature, rebuild, and verify that a body actually appeared.
    /// On failure the caller's snapshot rollback undoes the added feature.
    fn add_feature_and_rebuild(
        &mut self,
        feature: Feature,
        boolean_op: BooleanOp,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let feature_id = feature.id();
        let bodies_before = self.doc.cad.history.bodies().len();
        self.doc.cad.history.add_feature(feature);

        self.doc
            .cad
            .history
            .rebuild(&*self.kernel)
            .map_err(|e| EngineError::Feature(e.to_string()))?;

        // rebuild logs-and-continues on per-feature failure, so verify the
        // feature actually produced a body
        let bodies_after = self.doc.cad.history.bodies().len();
        if bodies_after <= bodies_before {
            let msg = if boolean_op == BooleanOp::Cut {
                "Cut operation is not supported by the Truck kernel. Only 'New Body', 'Join', and 'Intersect' are available.".to_string()
            } else if boolean_op != BooleanOp::New {
                format!(
                    "Boolean operation '{:?}' failed. Please check the target body and try again.",
                    boolean_op
                )
            } else {
                "No body was created. Check if the sketch has valid closed profiles.".to_string()
            };
            return Err(EngineError::Feature(msg));
        }

        events.push(Event::FeatureAdded { feature_id });
        events.push(Event::BodiesRebuilt {
            body_ids: self.body_ids(),
        });
        Ok(())
    }

    pub(crate) fn exec_delete_feature(
        &mut self,
        feature_id: Uuid,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        self.doc
            .cad
            .history
            .remove_feature(feature_id)
            .ok_or(EngineError::NotFound {
                kind: "feature",
                id: feature_id,
            })?;
        self.doc
            .cad
            .history
            .rebuild(&*self.kernel)
            .map_err(|e| EngineError::Feature(e.to_string()))?;
        events.push(Event::FeatureRemoved { feature_id });
        events.push(Event::BodiesRebuilt {
            body_ids: self.body_ids(),
        });
        Ok(())
    }

    pub(crate) fn exec_set_feature_suppressed(
        &mut self,
        feature_id: Uuid,
        suppressed: bool,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let feature =
            self.doc
                .cad
                .history
                .get_by_id_mut(feature_id)
                .ok_or(EngineError::NotFound {
                    kind: "feature",
                    id: feature_id,
                })?;
        feature.set_suppressed(suppressed);
        self.doc
            .cad
            .history
            .rebuild(&*self.kernel)
            .map_err(|e| EngineError::Feature(e.to_string()))?;
        events.push(Event::FeatureChanged { feature_id });
        events.push(Event::BodiesRebuilt {
            body_ids: self.body_ids(),
        });
        Ok(())
    }

    pub(crate) fn exec_rollback_to(
        &mut self,
        feature_id: Option<Uuid>,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        match feature_id {
            Some(id) => self
                .doc
                .cad
                .history
                .rollback_to(id)
                .map_err(|e| EngineError::Feature(e.to_string()))?,
            None => self.doc.cad.history.rollback_to_end(),
        }
        self.doc
            .cad
            .history
            .rebuild(&*self.kernel)
            .map_err(|e| EngineError::Feature(e.to_string()))?;
        if let Some(id) = feature_id {
            events.push(Event::FeatureChanged { feature_id: id });
        }
        events.push(Event::BodiesRebuilt {
            body_ids: self.body_ids(),
        });
        Ok(())
    }

    pub(crate) fn exec_rebuild_features(
        &mut self,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        self.doc
            .cad
            .history
            .rebuild(&*self.kernel)
            .map_err(|e| EngineError::Feature(e.to_string()))?;
        events.push(Event::BodiesRebuilt {
            body_ids: self.body_ids(),
        });
        Ok(())
    }

    // ================= Preview query =================

    /// Tessellated preview of an extrusion without touching the document,
    /// the undo history, or events.
    pub fn preview_extrude(
        &self,
        req: &ExtrudePreviewRequest,
    ) -> Result<TessellatedMesh, EngineError> {
        let sketch =
            self.doc
                .cad
                .history
                .get_sketch(req.sketch_id)
                .ok_or(EngineError::NotFound {
                    kind: "sketch",
                    id: req.sketch_id,
                })?;
        if req.profiles.is_empty() {
            return Err(EngineError::InvalidCommand("no profiles selected".into()));
        }

        let mut combined = extrude_solid(
            &*self.kernel,
            sketch,
            &req.profiles[0],
            req.distance,
            req.direction,
        )?;
        for profile in req.profiles.iter().skip(1) {
            let solid = extrude_solid(&*self.kernel, sketch, profile, req.distance, req.direction)?;
            combined = self
                .kernel
                .boolean(&combined, &solid, BooleanType::Union)
                .map_err(|e| EngineError::Cad(format!("boolean union failed: {e}")))?;
        }

        self.kernel
            .tessellate(&combined, DISPLAY_MESH_TOLERANCE)
            .map_err(|e| EngineError::Cad(format!("tessellation failed: {e}")))
    }
}

/// Extrude a single profile on the sketch plane (symmetric = two
/// extrusions unioned)
fn extrude_solid(
    kernel: &dyn CadKernel,
    sketch: &Sketch,
    profile: &Wire2D,
    distance: f32,
    direction: ExtrudeDirection,
) -> Result<Solid, EngineError> {
    if !kernel.is_available() {
        return Err(EngineError::Cad("CAD kernel not available".into()));
    }

    let extrude_dir = match direction {
        ExtrudeDirection::Positive | ExtrudeDirection::Symmetric => sketch.plane.normal,
        ExtrudeDirection::Negative => -sketch.plane.normal,
    };
    let extrude_dist = match direction {
        ExtrudeDirection::Symmetric => distance / 2.0,
        _ => distance,
    };

    let solid = kernel
        .extrude(
            profile,
            sketch.plane.origin,
            sketch.plane.x_axis,
            sketch.plane.y_axis,
            extrude_dir,
            extrude_dist,
        )
        .map_err(|e| EngineError::Cad(format!("extrude failed: {e}")))?;

    if matches!(direction, ExtrudeDirection::Symmetric) {
        let solid2 = kernel
            .extrude(
                profile,
                sketch.plane.origin,
                sketch.plane.x_axis,
                sketch.plane.y_axis,
                -extrude_dir,
                extrude_dist,
            )
            .map_err(|e| EngineError::Cad(format!("symmetric extrude failed: {e}")))?;
        kernel
            .boolean(&solid, &solid2, BooleanType::Union)
            .map_err(|e| EngineError::Cad(format!("boolean union failed: {e}")))
    } else {
        Ok(solid)
    }
}
