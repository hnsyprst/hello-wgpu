use std::cmp::{max, min};

use super::Collider;

pub struct SphereCollider {
    pub radius: f32,
    pub center: cgmath::Vector3<f32>,
}

impl SphereCollider {
    pub fn new(
        radius: f32,
        center: cgmath::Vector3<f32>,
    ) -> Self {
        Self {
            radius,
            center,
        }
    }

    pub fn update(
        &mut self,
        radius: f32,
        center: cgmath::Vector3<f32>,
    ) {
        self.radius = radius;
        self.center = center;
    }
}

impl Collider for SphereCollider {
    fn is_point_colliding(
        &self,
        point: &cgmath::Vector3<f32>,
    ) -> bool {
        // Is the distance from self.center to point smaller than self.radius?
        {
            // Multiplication is faster than pow
            let distance = (
                (point.x - self.center.x) * (point.x - self.center.x) +
                (point.y - self.center.y) * (point.y - self.center.y) +
                (point.z - self.center.z) * (point.z - self.center.z)
            ).sqrt();
            distance < self.radius
        }
    }

    fn is_aabb_colliding(
        &self,
        aabb: &super::aabb::AxisAlignedBoundingBoxCollider,
    ) -> bool {
        // Is the distance from self.centre to the AABB closest point less than self.radius?
        let closest = cgmath::Vector3::new(
            f32::max(aabb.min.x, f32::min(self.center.x, aabb.max.x)),
            f32::max(aabb.min.y, f32::min(self.center.y, aabb.max.y)),
            f32::max(aabb.min.z, f32::min(self.center.z, aabb.max.z)),
        );

        // Multiplication is faster than pow
        let distance = (
            (closest.x - self.center.x) * (closest.x - self.center.x) +
            (closest.y - self.center.y) * (closest.y - self.center.y) +
            (closest.z - self.center.z) * (closest.z - self.center.z)
        ).sqrt();
        distance < self.radius
    }

    fn is_sphere_colliding(
        &self,
        sphere: &SphereCollider,
    ) -> bool {
        // Is the distance between self.center and sphere.center less than or equal to the sum of each sphere's radii?
        let distance = (
            (self.center.x - sphere.center.x) * (self.center.x - sphere.center.x) +
            (self.center.y - sphere.center.y) * (self.center.y - sphere.center.y) +
            (self.center.z - sphere.center.z) * (self.center.z - sphere.center.z)
        ).sqrt();
        distance < self.radius + sphere.radius
    }
}