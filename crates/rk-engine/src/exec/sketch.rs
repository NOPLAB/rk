//! Sketch commands

use uuid::Uuid;

use rk_cad::{Sketch, SketchConstraint, SketchEntity, SketchPlane};

use crate::engine::Engine;
use crate::error::EngineError;
use crate::event::Event;

impl Engine {
    pub(crate) fn exec_create_sketch(
        &mut self,
        id: Option<Uuid>,
        name: Option<String>,
        plane: SketchPlane,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let name = name.unwrap_or_else(|| "Sketch".to_string());
        let sketch = match id {
            Some(id) => Sketch::with_id(id, name, plane),
            None => Sketch::new(name, plane),
        };
        let sketch_id = self.doc.cad.history.add_sketch(sketch);
        tracing::info!("created sketch: {}", sketch_id);
        events.push(Event::SketchAdded { sketch_id });
        Ok(())
    }

    pub(crate) fn exec_delete_sketch(
        &mut self,
        sketch_id: Uuid,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        self.doc
            .cad
            .history
            .remove_sketch(sketch_id)
            .ok_or(EngineError::NotFound {
                kind: "sketch",
                id: sketch_id,
            })?;
        events.push(Event::SketchRemoved { sketch_id });
        Ok(())
    }

    pub(crate) fn exec_rename_sketch(
        &mut self,
        sketch_id: Uuid,
        name: String,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let sketch = self.sketch_mut(sketch_id)?;
        sketch.name = name.clone();
        events.push(Event::SketchRenamed { sketch_id, name });
        Ok(())
    }

    pub(crate) fn exec_add_sketch_entities(
        &mut self,
        sketch_id: Uuid,
        entities: Vec<SketchEntity>,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let sketch = self.sketch_mut(sketch_id)?;
        for entity in entities {
            sketch.add_entity(entity);
        }
        events.push(Event::SketchGeometryChanged { sketch_id });
        Ok(())
    }

    pub(crate) fn exec_update_sketch_entity(
        &mut self,
        sketch_id: Uuid,
        entity: SketchEntity,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let entity_id = entity.id();
        let sketch = self.sketch_mut(sketch_id)?;
        if sketch.get_entity(entity_id).is_none() {
            return Err(EngineError::NotFound {
                kind: "sketch entity",
                id: entity_id,
            });
        }
        // add_entity keys by ID, so this replaces in place
        sketch.add_entity(entity);
        events.push(Event::SketchGeometryChanged { sketch_id });
        Ok(())
    }

    pub(crate) fn exec_delete_sketch_entities(
        &mut self,
        sketch_id: Uuid,
        entity_ids: Vec<Uuid>,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let sketch = self.sketch_mut(sketch_id)?;
        for entity_id in entity_ids {
            sketch.remove_entity(entity_id);
        }
        events.push(Event::SketchGeometryChanged { sketch_id });
        Ok(())
    }

    pub(crate) fn exec_add_sketch_constraint(
        &mut self,
        sketch_id: Uuid,
        constraint: SketchConstraint,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let sketch = self.sketch_mut(sketch_id)?;
        sketch.add_constraint(constraint)?;
        events.push(Event::SketchGeometryChanged { sketch_id });
        Ok(())
    }

    pub(crate) fn exec_delete_sketch_constraint(
        &mut self,
        sketch_id: Uuid,
        constraint_id: Uuid,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let sketch = self.sketch_mut(sketch_id)?;
        sketch
            .remove_constraint(constraint_id)
            .ok_or(EngineError::NotFound {
                kind: "sketch constraint",
                id: constraint_id,
            })?;
        events.push(Event::SketchGeometryChanged { sketch_id });
        Ok(())
    }

    pub(crate) fn exec_solve_sketch(
        &mut self,
        sketch_id: Uuid,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let sketch = self.sketch_mut(sketch_id)?;
        let result = sketch.solve();
        tracing::info!("sketch {} solve result: {:?}", sketch_id, result);
        events.push(Event::SketchSolved { sketch_id });
        events.push(Event::SketchGeometryChanged { sketch_id });
        Ok(())
    }

    pub(crate) fn exec_set_sketch_construction(
        &mut self,
        sketch_id: Uuid,
        entity_ids: Vec<Uuid>,
        construction: bool,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let sketch = self.sketch_mut(sketch_id)?;
        for id in &entity_ids {
            if sketch.get_entity(*id).is_none() {
                return Err(EngineError::NotFound {
                    kind: "sketch entity",
                    id: *id,
                });
            }
        }
        for id in entity_ids {
            sketch.set_construction(id, construction);
        }
        events.push(Event::SketchGeometryChanged { sketch_id });
        Ok(())
    }

    fn sketch_mut(&mut self, sketch_id: Uuid) -> Result<&mut Sketch, EngineError> {
        self.doc
            .cad
            .history
            .get_sketch_mut(sketch_id)
            .ok_or(EngineError::NotFound {
                kind: "sketch",
                id: sketch_id,
            })
    }
}
