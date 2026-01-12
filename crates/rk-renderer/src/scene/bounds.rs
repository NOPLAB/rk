//! Bounding box and frustum for culling.

use glam::{Mat4, Vec3};

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BoundingBox {
    /// Creates a new bounding box from min and max points.
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Creates an empty (inverted) bounding box.
    pub fn empty() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }

    /// Creates a bounding box from a center point and half-extents.
    pub fn from_center_half_extents(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    /// Creates a bounding box that contains all given points.
    pub fn from_points(points: impl IntoIterator<Item = Vec3>) -> Self {
        let mut bbox = Self::empty();
        for point in points {
            bbox = bbox.expand_to_include(point);
        }
        bbox
    }

    /// Returns the center of the bounding box.
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Returns the half-extents of the bounding box.
    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    /// Returns the size (full extents) of the bounding box.
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    /// Returns the radius of the bounding sphere.
    pub fn radius(&self) -> f32 {
        self.half_extents().length()
    }

    /// Returns true if the bounding box contains the given point.
    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    /// Returns true if this bounding box intersects another.
    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Returns the union of two bounding boxes.
    pub fn union(&self, other: &BoundingBox) -> BoundingBox {
        BoundingBox {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Returns a new bounding box expanded to include the given point.
    pub fn expand_to_include(&self, point: Vec3) -> BoundingBox {
        BoundingBox {
            min: self.min.min(point),
            max: self.max.max(point),
        }
    }

    /// Transforms the bounding box by the given matrix.
    ///
    /// Note: This returns an axis-aligned bounding box that contains
    /// the transformed corners, which may be larger than optimal.
    pub fn transform(&self, transform: &Mat4) -> BoundingBox {
        let corners = [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ];

        let transformed_corners = corners.map(|c| transform.transform_point3(c));
        BoundingBox::from_points(transformed_corners)
    }

    /// Returns true if the bounding box is valid (non-empty).
    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z
    }
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self::empty()
    }
}

/// Frustum for view culling.
#[derive(Debug, Clone, Copy)]
pub struct Frustum {
    planes: [Plane; 6],
}

/// A plane in 3D space (ax + by + cz + d = 0).
#[derive(Debug, Clone, Copy)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl Plane {
    /// Creates a new plane from normal and distance.
    pub fn new(normal: Vec3, distance: f32) -> Self {
        Self { normal, distance }
    }

    /// Creates a plane from a point and normal.
    pub fn from_point_normal(point: Vec3, normal: Vec3) -> Self {
        let n = normal.normalize();
        Self {
            normal: n,
            distance: -n.dot(point),
        }
    }

    /// Returns the signed distance from a point to the plane.
    pub fn distance_to_point(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }
}

impl Frustum {
    /// Creates a frustum from a view-projection matrix.
    pub fn from_view_proj(view_proj: Mat4) -> Self {
        let m = view_proj.to_cols_array_2d();

        // Extract frustum planes from the view-projection matrix
        // Using the method from "Fast Extraction of Viewing Frustum Planes
        // from the World-View-Projection Matrix" by Gil Gribb and Klaus Hartmann

        let planes = [
            // Left
            Plane::new(
                Vec3::new(m[0][3] + m[0][0], m[1][3] + m[1][0], m[2][3] + m[2][0]).normalize(),
                m[3][3] + m[3][0],
            ),
            // Right
            Plane::new(
                Vec3::new(m[0][3] - m[0][0], m[1][3] - m[1][0], m[2][3] - m[2][0]).normalize(),
                m[3][3] - m[3][0],
            ),
            // Bottom
            Plane::new(
                Vec3::new(m[0][3] + m[0][1], m[1][3] + m[1][1], m[2][3] + m[2][1]).normalize(),
                m[3][3] + m[3][1],
            ),
            // Top
            Plane::new(
                Vec3::new(m[0][3] - m[0][1], m[1][3] - m[1][1], m[2][3] - m[2][1]).normalize(),
                m[3][3] - m[3][1],
            ),
            // Near
            Plane::new(
                Vec3::new(m[0][3] + m[0][2], m[1][3] + m[1][2], m[2][3] + m[2][2]).normalize(),
                m[3][3] + m[3][2],
            ),
            // Far
            Plane::new(
                Vec3::new(m[0][3] - m[0][2], m[1][3] - m[1][2], m[2][3] - m[2][2]).normalize(),
                m[3][3] - m[3][2],
            ),
        ];

        Self { planes }
    }

    /// Returns true if a point is inside the frustum.
    pub fn contains_point(&self, point: Vec3) -> bool {
        for plane in &self.planes {
            if plane.distance_to_point(point) < 0.0 {
                return false;
            }
        }
        true
    }

    /// Returns true if a sphere intersects the frustum.
    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> bool {
        for plane in &self.planes {
            if plane.distance_to_point(center) < -radius {
                return false;
            }
        }
        true
    }

    /// Returns true if a bounding box intersects the frustum.
    pub fn intersects_box(&self, bbox: &BoundingBox) -> bool {
        for plane in &self.planes {
            // Find the vertex most in the direction of the plane normal
            let p = Vec3::new(
                if plane.normal.x >= 0.0 {
                    bbox.max.x
                } else {
                    bbox.min.x
                },
                if plane.normal.y >= 0.0 {
                    bbox.max.y
                } else {
                    bbox.min.y
                },
                if plane.normal.z >= 0.0 {
                    bbox.max.z
                } else {
                    bbox.min.z
                },
            );

            if plane.distance_to_point(p) < 0.0 {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounding_box_center() {
        let bbox = BoundingBox::new(Vec3::new(-1.0, -2.0, -3.0), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(bbox.center(), Vec3::ZERO);
    }

    #[test]
    fn test_bounding_box_half_extents() {
        let bbox = BoundingBox::new(Vec3::new(-1.0, -2.0, -3.0), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(bbox.half_extents(), Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_bounding_box_contains_point() {
        let bbox = BoundingBox::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert!(bbox.contains_point(Vec3::ZERO));
        assert!(bbox.contains_point(Vec3::new(0.5, 0.5, 0.5)));
        assert!(!bbox.contains_point(Vec3::new(2.0, 0.0, 0.0)));
    }

    #[test]
    fn test_bounding_box_union() {
        let a = BoundingBox::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(0.0, 0.0, 0.0));
        let b = BoundingBox::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let u = a.union(&b);
        assert_eq!(u.min, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(u.max, Vec3::new(1.0, 1.0, 1.0));
    }
}
