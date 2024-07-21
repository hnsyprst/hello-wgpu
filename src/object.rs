use crate::{model, instance};

pub struct Object<T> {
    pub model: T,
    pub instances: Vec<instance::Instance>,
}