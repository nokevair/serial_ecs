use std::convert::TryFrom;
use std::io;

use super::encode;
use super::decode;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ComponentIdx {
    // identifies the type of component
    pub(crate) id: u16,
    // the index of the component itself
    pub(crate) idx: u32,
}

pub(crate) struct EntityData {
    pub(crate) is_deleted: bool,
    pub(crate) components: Vec<ComponentIdx>,
}

pub(crate) struct EntityArray {
    pub(crate) entries: Vec<EntityData>,
}

impl EntityArray {
    // Compute the packed indices of the entries: this vector
    // contains `None` at indices corresponding to deleted
    // entities, and contains `Some(i)` everywhere else,
    // with the `i` values forming an increasing sequence.
    pub fn packed_idxs(&self) -> Vec<Option<u32>> {
        let mut idxs = Vec::with_capacity(self.entries.len());
        let mut i = 0;
        for entry in &self.entries {
            if entry.is_deleted {
                idxs.push(None);
            } else {
                idxs.push(Some(i));
                i += 1;
            }
        }
        idxs
    }
}

impl<R: io::Read> decode::State<R> {
    pub(crate) fn decode_component_idx(&mut self) -> Result<ComponentIdx, decode::Error> {
        let b = self.next("component index")?;
        let (id, idx) = match b {
            0x00 ..= 0x3f => (b as u16, self.decode_u8()? as u32),
            0x40 ..= 0x7f => ((b - 0x40) as u16, self.decode_u16()? as u32),
            0x80 => (self.decode_u8()? as u16, self.decode_u8()? as u32),
            0x81 => (self.decode_u8()? as u16, self.decode_u16()? as u32),
            0x82 => (self.decode_u8()? as u16, self.decode_u24()? as u32),
            0x83 => (self.decode_u8()? as u16, self.decode_u32()?),
            0x84 => (self.decode_u16()?, self.decode_u8()? as u32),
            0x85 => (self.decode_u16()?, self.decode_u16()? as u32),
            0x86 => (self.decode_u16()?, self.decode_u24()? as u32),
            0x87 => (self.decode_u16()?, self.decode_u32()?),
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

    pub(crate) fn decode_entity_data(&mut self) -> Result<EntityData, decode::Error> {
        let b = self.next("component index count")?;
        let num_comp_idxs = if b == 0xff { self.decode_u16()? } else { b as u16 };

        let mut components = Vec::with_capacity(num_comp_idxs as usize);
        for _ in 0..num_comp_idxs {
            components.push(self.decode_component_idx()?);
        }

        Ok(EntityData { is_deleted: false, components })
    }

    pub(crate) fn decode_entity_array(&mut self) -> Result<EntityArray, decode::Error> {
        let mut header = self.decode_header_line("entity array header")?;

        if header.len() != 2 {
            return Err(self.err_unexpected(
                "entity array header with two fields",
                format!("{} fields", header.len())
            ));
        }

        let signature = &header[0];
        if signature != "ENTITIES" {
            return Err(self.err_unexpected(
                "entity array signature (ENTITIES)",
                format!("invalid signature: {:?}", signature),
            ));
        }

        let num_entities = match header[1].parse::<u32>() {
            Ok(n) => n,
            Err(_) => return Err(self.err_unexpected(
                "32-bit entity count",
                "invalid entity count",
            )),
        };

        let mut entries = Vec::with_capacity(num_entities as usize);
        for _ in 0..num_entities {
            entries.push(self.decode_entity_data()?);
        }

        Ok(EntityArray { entries })
    }
}

enum IdScale {
    U6(u8),
    U8(u8),
    U16(u16),
}

enum IdxScale {
    Zero,
    U8(u8),
    U16(u16),
    U24(u32),
    U32(u32),
}

impl IdScale {
    fn from_id(id: u16) -> Self {
        if let Ok(id) = u8::try_from(id) {
            if id < 0x40 {
                Self::U6(id)
            } else {
                Self::U8(id)
            }
        } else {
            Self::U16(id)
        }
    }
}

impl IdxScale {
    fn from_idx(idx: u32) -> Self {
        if let Ok(idx) = u8::try_from(idx) {
            if idx == 0 {
                Self::Zero
            } else {
                Self::U8(idx)
            }
        } else if let Ok(idx) = u16::try_from(idx) {
            Self::U16(idx)
        } else if idx < (1 << 24) {
            Self::U24(idx)
        } else {
            Self::U32(idx)
        }
    }
}

impl<W: io::Write> encode::State<W> {
    pub(crate) fn encode_component_idx(&mut self, comp_idx: ComponentIdx) -> io::Result<()> {
        use IdScale as IdS;
        use IdxScale as IdxS;

        let id_s = IdS::from_id(comp_idx.id);
        let idx_s = IdxS::from_idx(comp_idx.idx as u32);

        /// Helper macro to call `self.write()` on the big-endian encoding of an integer.
        macro_rules! wi {
            (u24: $int:expr) => {{
                let [h, a, b, c] = $int.to_be_bytes();
                debug_assert_eq!(h, 0);
                self.write(&[a, b, c])?
            }};
            ($int:expr) => {
                self.write(&$int.to_be_bytes())?
            }
        }

        match (id_s, idx_s) {
            (IdS::U6(a), IdxS::Zero)   => { wi!(0xc0 + a) }
            (IdS::U6(a), IdxS::U8(b))  => { wi!(a); wi!(b) }
            (IdS::U6(a), IdxS::U16(b)) => { wi!(0x40 + a); wi!(b) }
            (IdS::U6(a), IdxS::U24(b)) => { wi!(0x82u8); wi!(a); wi!(u24: b) }
            (IdS::U6(a), IdxS::U32(b)) => { wi!(0x83u8); wi!(a); wi!(b) }

            (IdS::U8(a), IdxS::Zero)   => { wi!(0x88u8); wi!(a) }
            (IdS::U8(a), IdxS::U8(b))  => { wi!(0x80u8); wi!(a); wi!(b) }
            (IdS::U8(a), IdxS::U16(b)) => { wi!(0x81u8); wi!(a); wi!(b) }
            (IdS::U8(a), IdxS::U24(b)) => { wi!(0x82u8); wi!(a); wi!(u24: b) }
            (IdS::U8(a), IdxS::U32(b)) => { wi!(0x83u8); wi!(a); wi!(b) }

            (IdS::U16(a), IdxS::Zero)   => { wi!(0x89u8); wi!(a) }
            (IdS::U16(a), IdxS::U8(b))  => { wi!(0x84u8); wi!(a); wi!(b) }
            (IdS::U16(a), IdxS::U16(b)) => { wi!(0x85u8); wi!(a); wi!(b) }
            (IdS::U16(a), IdxS::U24(b)) => { wi!(0x86u8); wi!(a); wi!(u24: b) }
            (IdS::U16(a), IdxS::U32(b)) => { wi!(0x87u8); wi!(a); wi!(b) }
        }

        Ok(())
    }

    pub(crate) fn encode_entity_data(&mut self, data: &EntityData) -> io::Result<()> {
        let len = data.components.len();
        if len < 0xff {
            self.write(&[len as u8])?
        } else {
            debug_assert!(len < 0x10000, "entity cannot have >u16 components");
            self.write(&[0xff])?;
            self.write(&(len as u16).to_be_bytes())?
        }

        for &comp_idx in &data.components {
            self.encode_component_idx(comp_idx)?;
        }

        Ok(())
    }

    // WARNING: when calling this function, keep in mind that it skips deleted entities
    // when serializing, so if there are any components that contain `EntityId`s that refer
    // to this array, you need to remember to transform the component data using `Value::
    // mutate_entity_ids()` to correctly replaces the idxs with their packed versions.
    pub(crate) fn encode_entity_array(&mut self, array: &EntityArray) -> io::Result<()> {
        let len = array.entries.iter().filter(|e| !e.is_deleted).count();
        self.write_fmt(format_args!("ENTITIES {}\n", len))?;
        for entry in &array.entries {
            if entry.is_deleted {
                continue;
            }
            self.encode_entity_data(entry);
        }
        Ok(())
    }
}
