use bevy::prelude::Component;

#[derive(Component, Clone, PartialEq, Eq)]
pub enum Resource {
    Resource(String),
}

impl Resource {
    pub fn name(&self) -> &str {
        match self {
            Resource::Resource(name) => name,
        }
    }
}
