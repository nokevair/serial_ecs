use super::{Error, Result};
use super::parse::State;

#[derive(PartialEq, Debug)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Maybe(Option<Box<Value>>),
    EntityId(EntityId),
}

#[derive(PartialEq, Debug)]
pub enum EntityId {
    Invalid,
    Idx(u32),
}

impl<R: std::io::Read> State<R> {
    fn parse_bytes(&mut self, len: usize) -> Result<Value> {
        let mut bytes = Vec::new();
        for _ in 0..len {
            bytes.push(self.next("byte string")?);
        }
        Ok(Value::Bytes(bytes))
    }

    fn parse_array(&mut self, len: usize) -> Result<Value> {
        let mut vals = Vec::new();
        for _ in 0..len {
            vals.push(self.parse_value()?);
        }
        Ok(Value::Array(vals))
    }
    
    pub fn parse_value(&mut self) -> Result<Value> {
        let b = self.next("value")?;
        match b {
            0b00000000 ..= 0b01111111 => Ok(Value::Int(b as i64)),
            0b10000000 ..= 0b10001111 => self.parse_bytes((b - 0b10000000) as usize),
            0b10010000 ..= 0b10011111 => self.parse_array((b - 0b10010000) as usize),
            0b10100000 => { let len = self.parse_u8()?; self.parse_bytes(len as usize) }
            0b10100001 => { let len = self.parse_u32()?; self.parse_bytes(len as usize) }
            0b10100010 => { let len = self.parse_u8()?; self.parse_array(len as usize) }
            0b10100011 => { let len = self.parse_u32()?; self.parse_array(len as usize) }
            0b10100100 => Ok(Value::Bool(false)),
            0b10100101 => Ok(Value::Bool(true)),
            0b10100110 => Ok(Value::Float(self.parse_f32()? as f64)),
            0b10100111 => Ok(Value::Float(self.parse_f64()?)),
            0b10101000 => Ok(Value::Int(self.parse_i8()? as i64)),
            0b10101001 => Ok(Value::Int(self.parse_i16()? as i64)),
            0b10101010 => Ok(Value::Int(self.parse_i32()? as i64)),
            0b10101011 => Ok(Value::Int(self.parse_i64()?)),
            0b10101100 => Ok(Value::Maybe(None)),
            0b10101101 => Ok(Value::Maybe(Some(Box::new(self.parse_value()?)))),
            0b10101110 => Ok(Value::EntityId(EntityId::Idx(self.parse_u8()? as u32))),
            0b10101111 => Ok(Value::EntityId(EntityId::Idx(self.parse_u16()? as u32))),
            0b10110000 => Ok(Value::EntityId(EntityId::Idx(self.parse_u32()?))),
            0b10110001 => Ok(Value::EntityId(EntityId::Invalid)),
            _ => Err(Error::BadValueByte(b)),
        }
    }
}
