pub struct SphereCollider {
    radius: f32,
    center: cgmath::Vector3<f32>,
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

    pub fn is_point_colliding(
        &self,
        point: cgmath::Vector3<f32>,
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

    pub fn is_sphere_colliding(
        &self,
        sphere: &SphereCollider,
    ) -> bool {
        // Is the distance between self.center and sphere.center less than or equal to the sum of each sphere's radii?
        {
            let distance = (
                (self.center.x - sphere.center.x) * (self.center.x - sphere.center.x) +
                (self.center.y - sphere.center.y) * (self.center.y - sphere.center.y) +
                (self.center.z - sphere.center.z) * (self.center.z - sphere.center.z)
            ).sqrt();
            distance < self.radius + sphere.radius
        }
    }
}