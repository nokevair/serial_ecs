use std::io;

use super::decode;

use super::value::Value;

pub struct ComponentArray {
    name: String,
    id: u16,
    scheme: Vec<String>,
    values: Vec<Value>,
}

#[derive(Clone, Copy)]
pub struct ComponentRef<'a> {
    pub scheme: &'a [String],
    pub values: &'a [Value],
}

pub struct ComponentMut<'a> {
    pub scheme: &'a [String],
    pub values: &'a mut [Value],
}

impl ComponentArray {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn scheme(&self) -> &[String] {
        &self.scheme
    }

    pub fn is_empty(&self) -> bool {
        self.scheme.is_empty()
    }

    pub fn get(&self, idx: u32) -> Option<ComponentRef> {
        let scheme_len = self.scheme.len() as u32;
        let start = idx.checked_mul(scheme_len)? as usize;
        let end = idx.checked_add(1)?.checked_mul(scheme_len)? as usize;
        Some(ComponentRef {
            scheme: &self.scheme,
            values: self.values.get(start .. end)?,
        })
    }

    pub fn get_mut(&mut self, idx: u32) -> Option<ComponentMut> {
        let scheme_len = self.scheme.len() as u32;
        let start = idx.checked_mul(scheme_len)? as usize;
        let end = idx.checked_add(1)?.checked_mul(scheme_len)? as usize;
        Some(ComponentMut {
            scheme: &self.scheme,
            values: self.values.get_mut(start .. end)?,
        })
    }
}

impl<'a> ComponentRef<'a> {
    pub fn field_idx(self, name: &str) -> Option<usize> {
        self.scheme.iter().position(|n| n == name)
    }

    pub fn field(self, name: &str) -> Option<&'a Value> {
        Some(&self.values[self.field_idx(name)?])
    }
}

impl<'a> ComponentMut<'a> {
    pub fn field_idx(&self, name: &str) -> Option<usize> {
        self.scheme.iter().position(|n| n == name)
    }

    pub fn field(&'a self, name: &str) -> Option<&'a Value> {
        Some(&self.values[self.field_idx(name)?])
    }

    pub fn field_mut(&'a mut self, name: &str) -> Option<&'a mut Value> {
        Some(&mut self.values[self.field_idx(name)?])
    }
}

impl<R: io::Read> decode::State<R> {
    pub fn decode_component_array(&mut self) -> Result<ComponentArray, decode::Error> {
        let mut header = self.decode_header_line("component array header")?;

        // the first entry in the header should be the literal string `COMPONENT`
        if header.len() < 4 {
            return Err(self.err_unexpected(
                "component array header",
                "too few fields",
            ));
        }

        if header.remove(0) != "COMPONENT" {
            return Err(self.err_unexpected(
                "component array signature (COMPONENT)",
                "invalid signature",
            ));
        }
        
        // the second entry in the header should be the name of the component
        let name = header.remove(0);

        // the third entry is the ID of the component
        let id = match header.remove(0).parse::<u16>() {
            Ok(id) => id,
            Err(_) => return Err(self.err_unexpected(
                "16-bit component ID",
                "invalid ID",
            )),
        };

        // the fourth entry is the number of components
        let num_components = match header.remove(0).parse::<u32>() {
            Ok(n) => n,
            Err(_) => return Err(self.err_unexpected(
                "32-bit component count",
                "invalid component count",
            )),
        };

        // the rest of the entries describe the scheme
        let scheme = header;

        // decode the list of values comprising the component fields
        let num_values = num_components * scheme.len() as u32;
        let mut values = Vec::with_capacity(num_values as usize);
        for _ in 0..num_values {
            values.push(self.decode_value()?);
        }

        Ok(ComponentArray { name, id, scheme, values })
    }
}
