use rlua::Lua;

use std::cell::RefCell;
use std::io;
use std::rc::Rc;

use crate::decode;
use crate::encode;
use crate::error;
use crate::world::WorldData;

#[derive(Default)]
struct WorldDataRef(Rc<RefCell<WorldData>>);

pub struct World {
    lua: Lua,
    data: WorldDataRef,
}

impl World {
    pub fn new() -> Self {
        Self::with_lua(Lua::new())
    }

    pub fn with_lua(lua: Lua) -> Self {
        Self {
            lua,
            data: WorldDataRef::default(),
        }
    }

    pub fn from_reader<R: io::Read>(reader: R) -> Result<Self, error::DecodeError> {
        Self::from_reader_with_lua(reader, Lua::new())
    }

    pub fn from_reader_with_lua<R: io::Read>(
        reader: R,
        lua: Lua
    ) -> Result<Self, error::DecodeError> {
        let world_data = decode::State::new(reader).decode_world()?;
        Ok(Self {
            lua,
            data: WorldDataRef(Rc::new(RefCell::new(world_data))),
        })
    }

    pub fn to_writer<W: io::Write>(&self, writer: W) -> io::Result<()> {
        encode::State::new(writer)
            .encode_world(&self.data.0.borrow())
    }
}
