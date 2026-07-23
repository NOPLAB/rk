//! Assembly and joint commands

use glam::Vec3;
use uuid::Uuid;

use rk_core::{Joint, JointLimits, JointType, Link, Pose};

use crate::engine::Engine;
use crate::error::EngineError;
use crate::event::Event;

impl Engine {
    pub(crate) fn exec_connect_parts(
        &mut self,
        parent_part: Uuid,
        child_part: Uuid,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let parent_link_id = self.find_or_create_link(parent_part, events)?;
        let child_link_id = self.find_or_create_link(child_part, events)?;
        let assembly = &mut self.doc.project.assembly;

        // Reconnect: drop any existing parent first
        if assembly.parent.contains_key(&child_link_id)
            && let Err(e) = assembly.disconnect(child_link_id)
        {
            tracing::warn!("failed to disconnect existing parent: {}", e);
        }

        let parent_name = assembly
            .links
            .get(&parent_link_id)
            .map(|l| l.name.clone())
            .unwrap_or_default();
        let child_name = assembly
            .links
            .get(&child_link_id)
            .map(|l| l.name.clone())
            .unwrap_or_default();

        let joint = Joint::fixed(
            format!("{}_to_{}", parent_name, child_name),
            parent_link_id,
            child_link_id,
            Pose::default(),
        );

        let joint_id = assembly
            .connect(parent_link_id, child_link_id, joint)
            .map_err(|e| EngineError::InvalidCommand(e.to_string()))?;
        tracing::info!(
            "connected {} to {} via joint {}",
            parent_name,
            child_name,
            joint_id
        );
        events.push(Event::JointAdded {
            joint_id,
            parent_link: parent_link_id,
            child_link: child_link_id,
        });
        self.update_kinematics(events);
        Ok(())
    }

    pub(crate) fn exec_disconnect_part(
        &mut self,
        child_part: Uuid,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let link_id = self
            .link_for_part(child_part)
            .ok_or(EngineError::NotFound {
                kind: "link for part",
                id: child_part,
            })?;
        let joint = self
            .doc
            .project
            .assembly
            .disconnect(link_id)
            .map_err(|e| EngineError::InvalidCommand(e.to_string()))?;
        tracing::info!("disconnected part {}, removed joint {}", child_part, joint.name);
        events.push(Event::JointRemoved { joint_id: joint.id });
        self.update_kinematics(events);
        Ok(())
    }

    pub(crate) fn exec_set_joint_position(
        &mut self,
        joint_id: Uuid,
        position: f32,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let assembly = &mut self.doc.project.assembly;
        let joint = assembly.joints.get(&joint_id).ok_or(EngineError::NotFound {
            kind: "joint",
            id: joint_id,
        })?;
        let clamped = match &joint.limits {
            Some(limits) => position.clamp(limits.lower, limits.upper),
            None => position,
        };
        assembly.set_joint_position(joint_id, clamped);
        events.push(Event::JointPositionChanged {
            joint_id,
            position: clamped,
        });
        self.update_kinematics(events);
        Ok(())
    }

    pub(crate) fn exec_reset_joint_position(
        &mut self,
        joint_id: Uuid,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        self.doc.project.assembly.reset_joint_position(joint_id);
        events.push(Event::JointPositionChanged {
            joint_id,
            position: 0.0,
        });
        self.update_kinematics(events);
        Ok(())
    }

    pub(crate) fn exec_reset_all_joint_positions(
        &mut self,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        self.doc.project.assembly.reset_all_joint_positions();
        self.update_kinematics(events);
        Ok(())
    }

    pub(crate) fn exec_set_joint_type(
        &mut self,
        joint_id: Uuid,
        joint_type: JointType,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let joint = self.joint_mut(joint_id)?;
        joint.joint_type = joint_type;

        // Types that need limits get sensible defaults; others lose them
        if joint_type.has_limits() && joint.limits.is_none() {
            joint.limits = Some(if joint_type == JointType::Prismatic {
                JointLimits::default_prismatic()
            } else {
                JointLimits::default_revolute()
            });
        }
        if !joint_type.has_limits() {
            joint.limits = None;
        }

        events.push(Event::JointChanged { joint_id });
        self.update_kinematics(events);
        Ok(())
    }

    pub(crate) fn exec_set_joint_origin(
        &mut self,
        joint_id: Uuid,
        origin: Pose,
        keep_child_world_pose: bool,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        // Capture the child's world transform before the joint moves
        let child_part_info = if keep_child_world_pose {
            self.doc
                .project
                .assembly
                .get_joint(joint_id)
                .map(|j| j.child_link)
                .and_then(|child_id| {
                    self.doc.project.assembly.get_link(child_id).and_then(|link| {
                        link.part_id
                            .map(|part_id| (child_id, part_id, link.world_transform))
                    })
                })
        } else {
            None
        };

        let joint = self.joint_mut(joint_id)?;
        joint.origin = origin;

        self.doc
            .project
            .assembly
            .update_world_transforms_with_current_positions();

        // Compensate the child part's origin so only the joint moves,
        // not the mesh: new_origin = new_world⁻¹ * old_world * old_origin
        if let Some((child_id, part_id, old_child_world)) = child_part_info
            && let Some(new_child_link) = self.doc.project.assembly.get_link(child_id)
        {
            let compensation = new_child_link.world_transform.inverse() * old_child_world;
            if let Some(part) = self.doc.project.get_part_mut(part_id) {
                part.origin_transform = compensation * part.origin_transform;
            }
        }

        events.push(Event::JointChanged { joint_id });
        events.push(Event::WorldTransformsChanged {
            transforms: self.part_render_transforms(),
        });
        Ok(())
    }

    pub(crate) fn exec_set_joint_axis(
        &mut self,
        joint_id: Uuid,
        axis: Vec3,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let joint = self.joint_mut(joint_id)?;
        joint.axis = axis.normalize();
        events.push(Event::JointChanged { joint_id });
        self.update_kinematics(events);
        Ok(())
    }

    pub(crate) fn exec_set_joint_limits(
        &mut self,
        joint_id: Uuid,
        limits: Option<JointLimits>,
        events: &mut Vec<Event>,
    ) -> Result<(), EngineError> {
        let joint = self.joint_mut(joint_id)?;
        joint.limits = limits;
        events.push(Event::JointChanged { joint_id });

        // Clamp the current position into the new range
        if let Some(limits) = limits {
            let assembly = &mut self.doc.project.assembly;
            let current = assembly.get_joint_position(joint_id);
            let clamped = current.clamp(limits.lower, limits.upper);
            if clamped != current {
                assembly.set_joint_position(joint_id, clamped);
                events.push(Event::JointPositionChanged {
                    joint_id,
                    position: clamped,
                });
                self.update_kinematics(events);
            }
        }
        Ok(())
    }

    /// Find the link owning a part, creating one when absent
    fn find_or_create_link(
        &mut self,
        part_id: Uuid,
        events: &mut Vec<Event>,
    ) -> Result<Uuid, EngineError> {
        if let Some(link_id) = self.link_for_part(part_id) {
            return Ok(link_id);
        }
        let part = self
            .doc
            .project
            .get_part(part_id)
            .ok_or(EngineError::NotFound {
                kind: "part",
                id: part_id,
            })?;
        let link = Link::from_part(part);
        let link_id = self.doc.project.assembly.add_link(link);
        events.push(Event::LinkAdded {
            link_id,
            part_id: Some(part_id),
        });
        Ok(link_id)
    }

    pub(crate) fn link_for_part(&self, part_id: Uuid) -> Option<Uuid> {
        self.doc
            .project
            .assembly
            .links
            .iter()
            .find(|(_, l)| l.part_id == Some(part_id))
            .map(|(id, _)| *id)
    }

    fn joint_mut(&mut self, joint_id: Uuid) -> Result<&mut Joint, EngineError> {
        self.doc
            .project
            .assembly
            .get_joint_mut(joint_id)
            .ok_or(EngineError::NotFound {
                kind: "joint",
                id: joint_id,
            })
    }
}
