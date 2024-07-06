use crate::{model, instance};

pub struct Object {
    pub model: model::Model,
    pub instances: Vec<instance::Instance>,
}