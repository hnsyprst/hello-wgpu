pub struct AxisAlignedBoundingBox {
    pub min: cgmath::Vector3<f32>,
    pub max: cgmath::Vector3<f32>,
}

impl AxisAlignedBoundingBox {
    pub fn new(
        min: cgmath::Vector3<f32>,
        max: cgmath::Vector3<f32>,
    ) -> Self {
        Self {
            min,
            max,
        }
    }

    pub fn is_point_colliding(
        &self,
        point: cgmath::Vector3<f32>,
    ) -> bool {
        {
            point.x >= self.min.x &&
            point.x <= self.max.x &&
            point.y >= self.min.y &&
            point.y <= self.max.y &&
            point.z >= self.min.z &&
            point.z <= self.max.z
        }
    }

    pub fn is_aabb_colliding(
        &self,
        aabb: &AxisAlignedBoundingBox,
    ) -> bool {
        {
            aabb.min.x <= self.max.x &&
            aabb.max.x >= self.min.x &&
            aabb.min.y <= self.max.y &&
            aabb.max.y >= self.min.y &&
            aabb.min.z <= self.max.z &&
            aabb.max.z >= self.min.z
        }
    }
}