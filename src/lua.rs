use rlua::{Lua, RegistryKey};

use std::io;
use std::sync::{Arc, RwLock, PoisonError};

use crate::decode;
use crate::encode;
use crate::error;
use crate::world::WorldData;

#[derive(Default, Clone)]
struct WorldDataRef(Arc<RwLock<WorldData>>);

pub struct World {
    lua: Lua,
    data_ref_key: RegistryKey,

    data: WorldDataRef,
}

impl rlua::UserData for WorldDataRef {}

impl World {
    fn register_data_ref(
        lua: &Lua,
        data_ref: WorldDataRef
    ) -> rlua::Result<RegistryKey> {
        lua.context(|ctx| {
            ctx.create_registry_value(data_ref)
        })
    }

    pub fn new() -> Self {
        Self::with_lua(Lua::new())
    }

    pub fn with_lua(lua: Lua) -> Self {
        let data = WorldDataRef::default();
        let data_ref_key = Self::register_data_ref(&lua, data.clone())
            .expect("failed to add world data to Lua registry");
        Self {
            lua,
            data_ref_key,
            data,
        }
    }

    pub fn from_reader<R: io::Read>(reader: R) -> Result<Self, error::DecodeError> {
        Self::from_reader_with_lua(reader, Lua::new())
    }

    pub fn from_reader_with_lua<R: io::Read>(
        reader: R,
        lua: Lua
    ) -> Result<Self, error::DecodeError> {
        let data = decode::State::new(reader).decode_world()?;
        let data = WorldDataRef(Arc::new(RwLock::new(data)));
        let data_ref_key = Self::register_data_ref(&lua, data.clone())
            .expect("failed to add world data to Lua registry");
        
        Ok(Self { lua, data_ref_key, data })
    }

    pub fn to_writer<W: io::Write>(&self, writer: W) -> io::Result<()> {
        let world = self.data.0.read().unwrap_or_else(PoisonError::into_inner);
        encode::State::new(writer)
            .encode_world(&world)
    }
}
