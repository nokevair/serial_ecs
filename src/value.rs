use super::decode;

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

impl<R: std::io::Read> decode::State<R> {
    fn parse_bytes(&mut self, len: usize) -> Result<Value, decode::Error> {
        let mut bytes = Vec::new();
        for _ in 0..len {
            bytes.push(self.next("byte string")?);
        }
        Ok(Value::Bytes(bytes))
    }

    fn parse_array(&mut self, len: usize) -> Result<Value, decode::Error> {
        let mut vals = Vec::new();
        for _ in 0..len {
            vals.push(self.parse_value()?);
        }
        Ok(Value::Array(vals))
    }
    
    pub fn parse_value(&mut self) -> Result<Value, decode::Error> {
        let b = self.next("value")?;
        match b {
            0x00 ..= 0x7f => Ok(Value::Int(b as i64)),
            0x80 ..= 0x8f => self.parse_bytes((b - 0x80) as usize),
            0x90 ..= 0x9f => self.parse_array((b - 0x90) as usize),
            0xa0 => { let len = self.parse_u8()?; self.parse_bytes(len as usize) }
            0xa1 => { let len = self.parse_u32()?; self.parse_bytes(len as usize) }
            0xa2 => { let len = self.parse_u8()?; self.parse_array(len as usize) }
            0xa3 => { let len = self.parse_u32()?; self.parse_array(len as usize) }
            0xa4 => Ok(Value::Bool(false)),
            0xa5 => Ok(Value::Bool(true)),
            0xa6 => Ok(Value::Float(self.parse_f32()? as f64)),
            0xa7 => Ok(Value::Float(self.parse_f64()?)),
            0xa8 => Ok(Value::Int(self.parse_i8()? as i64)),
            0xa9 => Ok(Value::Int(self.parse_i16()? as i64)),
            0xaa => Ok(Value::Int(self.parse_i32()? as i64)),
            0xab => Ok(Value::Int(self.parse_i64()?)),
            0xac => Ok(Value::Maybe(None)),
            0xad => Ok(Value::Maybe(Some(Box::new(self.parse_value()?)))),
            0xae => Ok(Value::EntityId(EntityId::Idx(self.parse_u8()? as u32))),
            0xaf => Ok(Value::EntityId(EntityId::Idx(self.parse_u16()? as u32))),
            0xb0 => Ok(Value::EntityId(EntityId::Idx(self.parse_u32()?))),
            0xb1 => Ok(Value::EntityId(EntityId::Invalid)),

            0xb2 ..= 0xbf => Err(decode::Error::BadValueByte(b)),

            0xc0 ..= 0xff => Ok(Value::EntityId(EntityId::Idx((b - 0xc0) as u32))),
        }
    }
}
