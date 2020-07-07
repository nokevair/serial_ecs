mod decode;
mod encode;

mod entity;
mod world;

mod lua;

pub mod error;
pub mod value;
pub mod component;

pub use world::WorldData as WorldContext;
pub use lua::World;

#[cfg(test)]
mod test;
