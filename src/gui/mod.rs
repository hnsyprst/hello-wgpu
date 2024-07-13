use std::any::Any;

pub mod renderer;
pub mod windows;

pub type SendAny = dyn Any + Send;