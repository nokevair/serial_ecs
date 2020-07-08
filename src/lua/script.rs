use rlua::RegistryKey;

use super::WorldContext;

pub enum System {
    Lua(RegistryKey),
    Native(Box<dyn FnMut(&mut WorldContext)>),
}

pub enum Query<Q> {
    Lua(RegistryKey, Box<dyn FnMut(&rlua::Value) -> Q>),
    Native(Box<dyn FnMut(&mut WorldContext) -> Q>),
}
