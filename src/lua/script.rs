use rlua::RegistryKey;

use std::hash::Hash;

use super::{World, WorldContext};

pub enum ScriptType {
    Lua,
    Native,
    None,
}

pub enum System {
    Lua(RegistryKey),
    Native(Box<dyn FnMut(&mut WorldContext)>),
}

pub enum Query<Q> {
    Lua(RegistryKey, Box<dyn FnMut(rlua::Value) -> Q>),
    Native(Box<dyn FnMut(&mut WorldContext) -> Q>),
}

impl ScriptType {
    fn from_opt_system(sys: Option<&System>) -> Self {
        match sys {
            Some(System::Lua(_)) => Self::Lua,
            Some(System::Native(_)) => Self::Native,
            None => Self::None,
        }
    }

    fn from_opt_query<Q>(sys: Option<&Query<Q>>) -> Self {
        match sys {
            Some(Query::Lua(_, _)) => Self::Lua,
            Some(Query::Native(_)) => Self::Native,
            None => Self::None,
        }
    }
}

impl<ID, Q> World<ID, Q> where ID: Hash + Eq {
    pub fn register_lua_system(&mut self, id: ID, code: &[u8]) -> rlua::Result<ScriptType> {
        let key = self.lua.context(|ctx| {
            let system_fn: rlua::Function = ctx.load(code).eval()?;
            ctx.create_registry_value(system_fn)
        })?;
        let old = self.systems.insert(id, System::Lua(key));
        Ok(ScriptType::from_opt_system(old.as_ref()))
    }

    pub fn register_native_system(
        &mut self,
        id: ID,
        func: impl FnMut(&mut WorldContext) + 'static,
    ) -> ScriptType {
        let old = self.systems.insert(id, System::Native(Box::new(func)));
        ScriptType::from_opt_system(old.as_ref())
    }

    pub fn run_system(&mut self, id: &ID) -> rlua::Result<()> {
        match self.systems.get_mut(id) {
            None => Ok(()),
            Some(System::Lua(key)) => {
                let ctx_ref_key = &self.ctx_ref_key;
                self.lua.context(|ctx| {
                    let system_fn: rlua::Function = ctx.registry_value(key)?;
                    let ctx_ref: rlua::Value = ctx.registry_value(&ctx_ref_key)?;
                    let _: rlua::Value = system_fn.call(ctx_ref)?;
                    Ok(())
                })
            }
            Some(System::Native(ref mut func)) => {
                let mut world = self.ctx_ref.write();
                func(&mut *world);
                Ok(())
            }
        }
    }

    pub fn system_info(&self, id: &ID) -> ScriptType {
        ScriptType::from_opt_system(self.systems.get(id))
    }

    pub fn remove_system(&mut self, id: &ID) -> ScriptType {
        let old = self.systems.remove(id);
        ScriptType::from_opt_system(old.as_ref())
    }

    pub fn clear_systems(&mut self) {
        self.systems.clear();
        self.lua.context(|ctx| ctx.expire_registry_values());
    }

    pub fn register_lua_query(
        &mut self,
        id: ID,
        code: &[u8],
        post_process: impl FnMut(rlua::Value) -> Q + 'static,
    ) -> rlua::Result<ScriptType> {
        let key = self.lua.context(|ctx| {
            let query_fn: rlua::Function = ctx.load(code).eval()?;
            ctx.create_registry_value(query_fn)
        })?;
        let old = self.queries.insert(id, Query::Lua(key, Box::new(post_process)));
        Ok(ScriptType::from_opt_query(old.as_ref()))
    }

    pub fn register_native_query(
        &mut self,
        id: ID,
        func: impl FnMut(&mut WorldContext) -> Q + 'static
    ) -> ScriptType {
        let old = self.queries.insert(id, Query::Native(Box::new(func)));
        ScriptType::from_opt_query(old.as_ref())
    }

    pub fn run_query(&mut self, id: &ID) -> rlua::Result<Option<Q>> {
        match self.queries.get_mut(id) {
            None => Ok(None),
            Some(Query::Lua(key, post_process)) => {
                let ctx_ref_key = &self.ctx_ref_key;
                self.lua.context(|ctx| {
                    let system_fn: rlua::Function = ctx.registry_value(key)?;
                    let ctx_ref: rlua::Value = ctx.registry_value(&ctx_ref_key)?;
                    let result: rlua::Value = system_fn.call(ctx_ref)?;
                    Ok(Some(post_process(result)))
                })
            }
            Some(Query::Native(ref mut func)) => {
                let mut world = self.ctx_ref.write();
                Ok(Some(func(&mut *world)))
            }
        }
    }

    pub fn query_info(&self, id: &ID) -> ScriptType {
        ScriptType::from_opt_query(self.queries.get(id))
    }

    pub fn remove_query(&mut self, id: &ID) -> ScriptType {
        ScriptType::from_opt_query(self.queries.remove(id).as_ref())
    }

    pub fn clear_queries(&mut self) {
        self.systems.clear();
        self.lua.context(|ctx| ctx.expire_registry_values());
    }
}
