use super::Component;
use crate::EntityId;
#[derive(Clone, Debug)]
pub struct ParentComponent{
    pub parent: EntityId
}

impl ParentComponent{
    pub fn new(parent: EntityId) -> Self{
        Self{
            parent
        }
    }
}

impl Component for ParentComponent {}