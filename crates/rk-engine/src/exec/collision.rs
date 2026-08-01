//! Collision element commands. Collisions are addressed by (link, index)
//! because they live in a Vec on the link.

use uuid::Uuid;

use rk_core::{CollisionElement, GeometryType, Link, Pose};

use crate::engine::Engine;
use crate::error::EngineError;
use crate::event::Event;

impl Engine {
    pub(crate) fn exec_add_collision(
        &mut self,
        link_id: Uuid,
        geometry: GeometryType,
        origin: Pose,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let link = self.link_mut(link_id)?;
        link.collisions.push(CollisionElement {
            name: None,
            origin,
            geometry,
        });
        let index = link.collisions.len() - 1;
        events.push(Event::CollisionAdded { link_id, index });
        Ok(())
    }

    pub(crate) fn exec_remove_collision(
        &mut self,
        link_id: Uuid,
        index: usize,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let link = self.link_mut(link_id)?;
        if index >= link.collisions.len() {
            return Err(EngineError::InvalidCommand(format!(
                "collision index {index} out of bounds for link {link_id}"
            )));
        }
        link.collisions.remove(index);
        events.push(Event::CollisionRemoved { link_id, index });
        Ok(())
    }

    pub(crate) fn exec_set_collision_origin(
        &mut self,
        link_id: Uuid,
        index: usize,
        origin: Pose,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let collision = self.collision_mut(link_id, index)?;
        collision.origin = origin;
        events.push(Event::CollisionChanged { link_id, index });
        Ok(())
    }

    pub(crate) fn exec_set_collision_geometry(
        &mut self,
        link_id: Uuid,
        index: usize,
        geometry: GeometryType,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let collision = self.collision_mut(link_id, index)?;
        collision.geometry = geometry;
        events.push(Event::CollisionChanged { link_id, index });
        Ok(())
    }

    fn link_mut(&mut self, link_id: Uuid) -> Result<&mut Link, EngineError> {
        self.doc
            .project
            .assembly
            .get_link_mut(link_id)
            .ok_or(EngineError::NotFound {
                kind: "link",
                id: link_id,
            })
    }

    fn collision_mut(
        &mut self,
        link_id: Uuid,
        index: usize,
    ) -> Result<&mut CollisionElement, EngineError> {
        self.link_mut(link_id)?
            .collisions
            .get_mut(index)
            .ok_or_else(|| {
                EngineError::InvalidCommand(format!(
                    "collision index {index} out of bounds for link {link_id}"
                ))
            })
    }
}
