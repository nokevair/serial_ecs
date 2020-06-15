# Serial ECS

`serial_ecs` is an efficient, dynamic rust-based ECS architecture. It supports:

- Components which contain an arbitrary number of dynamically-typed values.
- Entities which contain an arbitrary number of components.
- Serialization and deserialization of entity and component arrays in a compact binary format.
- Iteration over all components of a given type.
- Iteration over all entities containing a given set of components.
- Direct manipulation of entities and components through a Rust-based API.
- Registration of scripts, called *systems*, which can manipulate entities and components natively. These can be written in Rust or Lua.
