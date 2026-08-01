//! Part commands

use glam::Mat4;
use uuid::Uuid;

use rk_core::{
    InertiaMatrix, Part, generate_box_mesh, generate_cylinder_mesh, generate_sphere_mesh,
};

use crate::command::PrimitiveSpec;
use crate::engine::Engine;
use crate::error::EngineError;
use crate::event::Event;

impl Engine {
    pub(crate) fn exec_create_primitive(
        &mut self,
        id: Option<Uuid>,
        primitive: PrimitiveSpec,
        name: Option<String>,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let existing_count = self.doc.project.parts().len();
        let part_name =
            name.unwrap_or_else(|| format!("{}_{}", primitive.type_name(), existing_count + 1));

        let (vertices, normals, indices) = match primitive {
            PrimitiveSpec::Box { size } => generate_box_mesh(size),
            PrimitiveSpec::Cylinder { radius, height } => generate_cylinder_mesh(radius, height),
            PrimitiveSpec::Sphere { radius } => generate_sphere_mesh(radius),
        };

        let mut part = Part::new(&part_name);
        if let Some(id) = id {
            part.id = id;
        }
        part.vertices = vertices;
        part.normals = normals;
        part.indices = indices;
        part.calculate_bounding_box();
        part.material_name = Some(format!("{}_material", primitive.type_name().to_lowercase()));

        tracing::info!(
            "created primitive: {} ({} vertices)",
            part.name,
            part.vertices.len()
        );
        let part_id = self.doc.project.add_part(part);
        events.push(Event::PartAdded { part_id });
        Ok(())
    }

    pub(crate) fn exec_create_empty_part(
        &mut self,
        id: Option<Uuid>,
        name: Option<String>,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let existing_count = self.doc.project.parts().len();
        let part_name = name.unwrap_or_else(|| format!("Empty_{}", existing_count + 1));
        let mut part = Part::new(&part_name);
        if let Some(id) = id {
            part.id = id;
        }
        let part_id = self.doc.project.add_part(part);
        events.push(Event::PartAdded { part_id });
        Ok(())
    }

    pub(crate) fn exec_delete_part(
        &mut self,
        part_id: Uuid,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        self.doc
            .project
            .remove_part(part_id)
            .ok_or(EngineError::NotFound {
                kind: "part",
                id: part_id,
            })?;
        events.push(Event::PartRemoved { part_id });
        Ok(())
    }

    pub(crate) fn exec_rename_part(
        &mut self,
        part_id: Uuid,
        name: String,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let part = self.part_mut(part_id)?;
        part.name = name.clone();
        events.push(Event::PartRenamed { part_id, name });
        Ok(())
    }

    pub(crate) fn exec_set_part_transform(
        &mut self,
        part_id: Uuid,
        transform: Mat4,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let part = self.part_mut(part_id)?;
        part.origin_transform = transform;
        self.update_kinematics(events);
        Ok(())
    }

    pub(crate) fn exec_set_part_color(
        &mut self,
        part_id: Uuid,
        color: [f32; 4],
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let part = self.part_mut(part_id)?;
        part.color = color;
        events.push(Event::PartAppearanceChanged { part_id });
        Ok(())
    }

    pub(crate) fn exec_set_part_material(
        &mut self,
        part_id: Uuid,
        material_name: Option<String>,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let part = self.part_mut(part_id)?;
        part.material_name = material_name;
        events.push(Event::PartAppearanceChanged { part_id });
        Ok(())
    }

    pub(crate) fn exec_set_part_mass(
        &mut self,
        part_id: Uuid,
        mass: f32,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let part = self.part_mut(part_id)?;
        part.mass = mass;
        self.sync_link_inertial(part_id);
        events.push(Event::PartPhysicsChanged { part_id });
        Ok(())
    }

    pub(crate) fn exec_set_part_inertia(
        &mut self,
        part_id: Uuid,
        inertia: InertiaMatrix,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let part = self.part_mut(part_id)?;
        part.inertia = inertia;
        self.sync_link_inertial(part_id);
        events.push(Event::PartPhysicsChanged { part_id });
        Ok(())
    }

    /// `Link::from_part` copies mass and inertia into the link, and URDF
    /// export writes that copy — so later edits to the part have to follow.
    fn sync_link_inertial(&mut self, part_id: Uuid) {
        let Some(part) = self.doc.project.get_part(part_id) else {
            return;
        };
        let (mass, inertia) = (part.mass, part.inertia);
        let Some(link_id) = self.link_for_part(part_id) else {
            return;
        };
        if let Some(link) = self.doc.project.assembly.links.get_mut(&link_id) {
            link.inertial.mass = mass;
            link.inertial.inertia = inertia;
        }
    }

    fn part_mut(&mut self, part_id: Uuid) -> Result<&mut Part, EngineError> {
        self.doc
            .project
            .get_part_mut(part_id)
            .ok_or(EngineError::NotFound {
                kind: "part",
                id: part_id,
            })
    }
}
