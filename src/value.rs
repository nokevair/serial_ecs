pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Null,
    Just(Box<Value>),
    EntityId(EntityId),
}

pub enum EntityId {
    Invalid,
    Idx(usize),
}
