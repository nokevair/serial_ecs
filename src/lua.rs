use rlua::{Lua, RegistryKey};

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard, PoisonError};

use crate::decode;
use crate::encode;
use crate::error;
use crate::WorldContext;

mod script;
use script::{System, Query};

pub use script::ScriptType;

#[derive(Default, Clone)]
struct ContextRef(Arc<RwLock<WorldContext>>);

pub struct World<ID, Q> {
    lua: Lua,
    ctx_ref_key: RegistryKey,

    systems: HashMap<ID, System>,
    queries: HashMap<ID, Query<Q>>,

    ctx_ref: ContextRef,
}

impl ContextRef {
    fn read(&self) -> RwLockReadGuard<WorldContext> {
        self.0.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn write(&self) -> RwLockWriteGuard<WorldContext> {
        self.0.write().unwrap_or_else(PoisonError::into_inner)
    }
}

impl rlua::UserData for ContextRef {}

impl<ID, Q> World<ID, Q> {
    fn from_ctx_ref_with_lua(
        ctx_ref: ContextRef,
        lua: Lua
    ) -> Self {
        let ctx_ref_key = lua.context(|ctx|
            ctx.create_registry_value(ctx_ref.clone())
                .expect("failed to add world data to Lua registry"));
        Self {
            lua,
            ctx_ref_key,

            systems: HashMap::new(),
            queries: HashMap::new(),

            ctx_ref,
        }
    }

    pub fn new() -> Self {
        Self::with_lua(Lua::new())
    }

    pub fn with_lua(lua: Lua) -> Self {
        Self::from_ctx_ref_with_lua(ContextRef::default(), lua)
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
        
        Ok(Self::from_ctx_ref_with_lua(ctx_ref, lua))
    }

    pub fn to_writer<W: io::Write>(&self, writer: W) -> io::Result<()> {
        let world = self.ctx_ref.read();
        encode::State::new(writer)
            .encode_world(&world)
    }
}
