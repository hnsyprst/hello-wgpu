use super::Collider;

pub struct AxisAlignedBoundingBoxCollider {
    pub min: cgmath::Vector3<f32>,
    pub max: cgmath::Vector3<f32>,
}

impl AxisAlignedBoundingBoxCollider {
    pub fn new(
        half_size: cgmath::Vector3<f32>,
        center: cgmath::Vector3<f32>,
    ) -> Self {
        Self {
            min: -half_size + center,
            max: half_size + center,
        }
    }

    pub fn update(
        &mut self,
        half_size: cgmath::Vector3<f32>,
        center: cgmath::Vector3<f32>,
    ) {
        self.min = -half_size + center;
        self.max = half_size + center;
    }
}

impl Collider for AxisAlignedBoundingBoxCollider {
    fn is_point_colliding(
        &self,
        point: &cgmath::Vector3<f32>,
    ) -> bool {
        // Does the point lie inside self?
        {
            point.x >= self.min.x &&
            point.x <= self.max.x &&
            point.y >= self.min.y &&
            point.y <= self.max.y &&
            point.z >= self.min.z &&
            point.z <= self.max.z
        }
    }

    fn is_aabb_colliding(
        &self,
        aabb: &AxisAlignedBoundingBoxCollider,
    ) -> bool {
        // Do the ranges self.min-self.max and aabb.min-aabb.max overlap?
        {
            aabb.min.x <= self.max.x &&
            aabb.max.x >= self.min.x &&
            aabb.min.y <= self.max.y &&
            aabb.max.y >= self.min.y &&
            aabb.min.z <= self.max.z &&
            aabb.max.z >= self.min.z
        }
    }

    fn is_sphere_colliding(
        &self,
        sphere: &super::sphere::SphereCollider,
    ) -> bool {
        // Is the distance from my closest point to the sphere's centre less than the sphere's radius?
        let closest = cgmath::Vector3::new(
            f32::max(self.min.x, f32::min(sphere.center.x, self.max.x)),
            f32::max(self.min.y, f32::min(sphere.center.y, self.max.y)),
            f32::max(self.min.z, f32::min(sphere.center.z, self.max.z)),
        );

        // Multiplication is faster than pow
        let distance = (
            (closest.x - sphere.center.x) * (closest.x - sphere.center.x) +
            (closest.y - sphere.center.y) * (closest.y - sphere.center.y) +
            (closest.z - sphere.center.z) * (closest.z - sphere.center.z)
        ).sqrt();
        distance < sphere.radius
    }
}