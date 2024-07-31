pub mod aabb;
pub mod sphere;

pub enum ColliderEnum {
    AABB(aabb::AxisAlignedBoundingBoxCollider),
    Sphere(sphere::SphereCollider),
}

pub trait Collider {
    fn is_point_colliding(
        &self,
        point: &cgmath::Vector3<f32>,
    ) -> bool;

    fn is_aabb_colliding(
        &self,
        aabb: &aabb::AxisAlignedBoundingBoxCollider,
    ) -> bool;

    fn is_sphere_colliding(
        &self,
        sphere: &sphere::SphereCollider,
    ) -> bool;

    fn is_colliding_with(
        &self,
        other: &ColliderEnum,
    ) -> bool {
        match other {
            ColliderEnum::AABB(aabb) => self.is_aabb_colliding(aabb),
            ColliderEnum::Sphere(sphere) => self.is_sphere_colliding(sphere),
        }
    }
}