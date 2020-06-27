use vec_map::VecMap;

use std::collections::HashSet;
use std::io;

use super::decode;
use super::encode;

use super::error;
use super::value::EntityId;

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

    pub fn from_reader<R: io::Read>(reader: R) -> Result<Self, error::DecodeError> {
        decode::State::new(reader)
            .decode_world()
    }

    pub fn to_writer<W: io::Write>(&self, writer: W) -> io::Result<()> {
        encode::State::new(writer)
            .encode_world(self)
    }
}

impl<R: io::Read> decode::State<R> {
    pub fn decode_world(&mut self) -> Result<World, decode::Error> {
        let mut header = self.decode_header_line("world state header")?;

        if header.len() != 3 {
            return Err(self.err_unexpected(
                "world state header with three fields",
                format!("{} fields", header.len()),
            ));
        }

        let signature = &header[0];
        if signature != "WORLD" {
            return Err(self.err_unexpected(
                "world state signature (WORLD)",
                format!("invalid signature: {:?}", signature),
            ));
        }

        let num_component_arrays = match header[1].parse::<u16>() {
            Ok(n) => n,
            Err(_) => return Err(self.err_unexpected(
                "16-bit entity array count",
                "invalid entity array count",
            )),
        };

        let max_component_id = match header[2].parse::<u16>() {
            Ok(n) => n,
            Err(_) => return Err(self.err_unexpected(
                "16-bit maximum component ID",
                "invalid maximum component ID",
            ))
        };
        
        let mut component_arrays = VecMap::with_capacity(max_component_id as usize);
        let mut component_names = HashSet::with_capacity(max_component_id as usize);

        // Read a sequence of component arrays
        for _ in 0..num_component_arrays {
            let array = self.decode_component_array()?;
            let id = array.id();
            let name = array.name();

            if !component_names.insert(name.to_string()) {
                return Err(self.err_unexpected(
                    "unique component names",
                    format!("duplicate component name {:?}", name),
                ));
            }
            if id > max_component_id {
                return Err(self.err_unexpected(
                    format!("all component IDs within the maximum specified ({})",
                        max_component_id),
                    format!("component {:?} with ID greater than the maximum ({})",
                        name, id),
                ));
            }
            if component_arrays.contains_key(id as usize) {
                return Err(self.err_unexpected(
                    "unique component IDs",
                    format!("component {:?} with duplicate ID: {}", name, id),
                ));
            }

            component_arrays.insert(id as usize, array);
            self.expect_newline()?;
        }

        let global = self.decode_global_component()?;
        self.expect_newline()?;

        let entities = self.decode_entity_array()?;

        Ok(World { components: component_arrays, global, entities })
    }
}

impl<W: io::Write> encode::State<W> {
    pub fn encode_world(&mut self, world: &World) -> io::Result<()> {
        let num_component_arrays = world.components.len();
        let max_component_arrays = world.components.iter()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        
        self.write_fmt(format_args!(
            "WORLD {} {}\n",
            num_component_arrays,
            max_component_arrays,
        ))?;

        let packed_idxs = world.entities.packed_idxs();
        let transform_id = |id: &mut EntityId| {
            if let EntityId::Idx(ref mut idx) = id {
                if let Some(&Some(new_idx)) = packed_idxs.get(*idx as usize) {
                    *idx = new_idx;
                } else {
                    *id = EntityId::Invalid;
                }
            }
        };

        // Encode the component arrays, but transform any `EntityId`s they contain
        // to reflect the fact that deleted entities are not serialzed.
        for component_array in world.components.values() {
            self.encode_component_array(component_array, transform_id)?;
            self.write(b"\n")?;
        }

        // Encode the global component, applying the same transformation.
        self.encode_global_component(&world.global, transform_id)?;
        self.write(b"\n")?;

        // Encode the entity array.
        self.encode_entity_array(&world.entities)?;

        Ok(())
    }
}
