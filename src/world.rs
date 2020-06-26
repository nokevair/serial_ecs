use vec_map::VecMap;

use super::component::{ComponentArray, GlobalComponent};
use super::entity::EntityArray;

pub struct World {
    components: VecMap<ComponentArray>,
    global: GlobalComponent,
    entities: EntityArray,
}

impl World {
    pub fn empty() -> Self {
        Self {
            components: VecMap::new(),
            global: GlobalComponent::empty(),
            entities: EntityArray::empty(),
        }
    }
}
