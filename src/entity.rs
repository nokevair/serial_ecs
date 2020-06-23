use std::io;

use super::encode;
use super::decode;

pub(crate) struct ComponentIdx {
    // identifies the type of component
    id: u16,
    // the index of the component itself
    idx: usize,
}

struct EntityData {
    components: Vec<ComponentIdx>,
}

impl<R: io::Read> decode::State<R> {
    pub(crate) fn decode_component_idx(&mut self) -> Result<ComponentIdx, decode::Error> {
        let b = self.next("component index")?;
        let (id, idx): (u16, usize) = match b {
            0x00 ..= 0x3f => (b as u16, self.decode_u8()? as usize),
            0x40 ..= 0x7f => ((b - 0x40) as u16, self.decode_u16()? as usize),
            0x80 => (self.decode_u8()? as u16, self.decode_u8()? as usize),
            0x81 => (self.decode_u8()? as u16, self.decode_u16()? as usize),
            0x82 => (self.decode_u8()? as u16, self.decode_u24()? as usize),
            0x83 => (self.decode_u8()? as u16, self.decode_u32()? as usize),
            0x84 => (self.decode_u16()?, self.decode_u8()? as usize),
            0x85 => (self.decode_u16()?, self.decode_u16()? as usize),
            0x86 => (self.decode_u16()?, self.decode_u24()? as usize),
            0x87 => (self.decode_u16()?, self.decode_u32()? as usize),
            0x88 => (self.decode_u8()? as u16, 0),
            0x89 => (self.decode_u16()? as u16, 0),

            0x8a ..= 0xbf => return Err(self.err_unexpected(
                "component index",
                format!("invalid byte ({:?})", b),
            )),

            0xc0 ..= 0xff => ((b - 0xc0) as u16, 0),
        };
        Ok(ComponentIdx { id, idx })
    }
}