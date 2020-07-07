use rlua::{Lua, RegistryKey};

use std::io;
use std::sync::{Arc, RwLock, PoisonError};

use crate::decode;
use crate::encode;
use crate::error;
use crate::WorldContext;

#[derive(Default, Clone)]
struct ContextRef(Arc<RwLock<WorldContext>>);

pub struct World {
    lua: Lua,
    ctx_ref_key: RegistryKey,

    ctx_ref: ContextRef,
}

impl rlua::UserData for ContextRef {}

impl World {
    fn register_ctx_ref(
        lua: &Lua,
        data_ref: ContextRef,
    ) -> rlua::Result<RegistryKey> {
        lua.context(|ctx|
            ctx.create_registry_value(data_ref))
    }

    pub fn new() -> Self {
        Self::with_lua(Lua::new())
    }

    pub fn with_lua(lua: Lua) -> Self {
        let ctx_ref = ContextRef::default();
        let ctx_ref_key = Self::register_ctx_ref(&lua, ctx_ref.clone())
            .expect("failed to add world data to Lua registry");
        Self {
            lua,
            ctx_ref_key,
            ctx_ref,
        }
    }

    pub fn from_reader<R: io::Read>(reader: R) -> Result<Self, error::DecodeError> {
        Self::from_reader_with_lua(reader, Lua::new())
    }

    pub fn from_reader_with_lua<R: io::Read>(
        reader: R,
        lua: Lua
    ) -> Result<Self, error::DecodeError> {
        let ctx = decode::State::new(reader).decode_world()?;
        let ctx_ref = ContextRef(Arc::new(RwLock::new(ctx)));
        let ctx_ref_key = Self::register_ctx_ref(&lua, ctx_ref.clone())
            .expect("failed to add world data to Lua registry");
        
        Ok(Self { lua, ctx_ref_key, ctx_ref })
    }

    pub fn to_writer<W: io::Write>(&self, writer: W) -> io::Result<()> {
        let world = self.ctx.0.read().unwrap_or_else(PoisonError::into_inner);
        encode::State::new(writer)
            .encode_world(&world)
    }
}
