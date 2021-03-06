use std::io;

use super::encode;
use super::decode;

use super::value::{Value, EntityId};

// Find the first duplicate in `vals` using an `O(n^2)` algorithm.
// This should probably only be used on small arrays.
fn find_duplicate_quadratic<T: Eq>(ts: &[T]) -> Option<&T> {
    ts.iter().enumerate().find_map(
        |(i, t)| if ts[..i].contains(t) {
            Some(t)
        } else {
            None
        })
}

pub struct ComponentArray {
    name: String,
    id: u16,
    scheme: Vec<String>,
    values: Vec<Value>,
}

pub struct GlobalComponent {
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

    pub fn is_marker(&self) -> bool {
        self.scheme.is_empty()
    }

    pub fn field_idx(&self, name: &str) -> Option<usize> {
        self.scheme.iter().position(|n| n == name)
    }

    pub fn get(&self, idx: u32) -> Option<ComponentRef> {
        let scheme_len = self.scheme.len() as u32;
        if scheme_len == 0 && idx != 0 { return None; }
        let start = idx.checked_mul(scheme_len)? as usize;
        let end = idx.checked_add(1)?.checked_mul(scheme_len)? as usize;
        Some(ComponentRef {
            scheme: &self.scheme,
            values: self.values.get(start .. end)?,
        })
    }

    pub fn get_mut(&mut self, idx: u32) -> Option<ComponentMut> {
        let scheme_len = self.scheme.len() as u32;
        if scheme_len == 0 && idx != 0 { return None; }
        let start = idx.checked_mul(scheme_len)? as usize;
        let end = idx.checked_add(1)?.checked_mul(scheme_len)? as usize;
        Some(ComponentMut {
            scheme: &self.scheme,
            values: self.values.get_mut(start .. end)?,
        })
    }
}

impl GlobalComponent {
    pub fn empty() -> Self {
        Self {
            scheme: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn scheme(&self) -> &[String] {
        &self.scheme
    }

    pub fn is_empty(&self) -> bool {
        self.scheme.is_empty()
    }

    pub fn field_idx(&self, name: &str) -> Option<usize> {
        self.scheme.iter().position(|n| n == name)
    }

    pub fn get(&self) -> ComponentRef {
        ComponentRef {
            scheme: &self.scheme,
            values: &self.values,
        }
    }

    pub fn get_mut(&mut self) -> ComponentMut {
        ComponentMut {
            scheme: &self.scheme,
            values: &mut self.values,
        }
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

    pub fn to_ref(&'a self) -> ComponentRef<'a> {
        ComponentRef {
            scheme: &self.scheme,
            values: &self.values,
        }
    }
}

impl<R: io::Read> decode::State<R> {
    pub fn decode_component_array(&mut self) -> Result<ComponentArray, decode::Error> {
        let mut header = self.decode_header_line("component array header")?;

        if header.len() < 4 {
            return Err(self.err_unexpected(
                "component array header",
                "too few fields",
            ));
        }

        // the first entry in the header should be the literal string `COMPONENT`
        let signature = header.remove(0);
        if signature != "COMPONENT" {
            return Err(self.err_unexpected(
                "component array signature (COMPONENT)",
                format!("invalid signature: {:?}", signature),
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
        
        // ensure that the scheme has no duplicate fields
        if let Some(dup) = find_duplicate_quadratic(&scheme) {
            return Err(self.err_unexpected(
                "distinct field names",
                format!("duplicate name: {:?}", dup),
            ))
        }

        // decode the list of values comprising the component fields
        let num_values = num_components * scheme.len() as u32;
        let mut values = Vec::with_capacity(num_values as usize);
        for _ in 0..num_values {
            values.push(self.decode_value()?);
        }

        Ok(ComponentArray { name, id, scheme, values })
    }

    pub fn decode_global_component(&mut self) -> Result<GlobalComponent, decode::Error> {
        let mut header = self.decode_header_line("global component header")?;
        
        if header.is_empty() {
            return Err(self.err_unexpected(
                "global component header",
                "too few fields",
            ));
        }

        // the first entry in the header should be the literal string "GLOBAL"
        let signature = header.remove(0);
        if signature != "GLOBAL" {
            return Err(self.err_unexpected(
                "global component signature (GLOBAL)",
                format!("invalid signature: {:?}", signature),
            ));
        }

        let scheme = header;

        // ensure that the scheme has no duplicate fields
        if let Some(dup) = find_duplicate_quadratic(&scheme) {
            return Err(self.err_unexpected(
                "distinct field names",
                format!("duplicate name: {:?}", dup),
            ));
        }

        let num_values = scheme.len();
        let mut values = Vec::with_capacity(num_values);
        for _ in 0..num_values {
            values.push(self.decode_value()?);
        }

        Ok(GlobalComponent { scheme, values })
    }
}

impl<W: io::Write> encode::State<W> {
    pub fn encode_component_array<ET: FnMut(&mut EntityId)>(
        &mut self,
        array: &ComponentArray,
        mut e_id_transform: ET,
    ) -> io::Result<()> {
        let len = array.values.len()
            .checked_div(array.scheme.len())
            .unwrap_or(0);
        self.write_fmt(format_args!("COMPONENT {} {} {}", array.name, array.id, len))?;
        for field_name in &array.scheme {
            self.write(b" ")?;
            self.write(field_name.as_bytes())?;
        }
        self.write(b"\n")?;
        for value in &array.values {
            self.encode_value(value, &mut e_id_transform)?;
        }
        Ok(())
    }

    pub fn encode_global_component<ET: FnMut(&mut EntityId)>(
        &mut self,
        global: &GlobalComponent,
        mut e_id_transform: ET,
    ) -> io::Result<()> {
        self.write(b"GLOBAL")?;
        for field_name in &global.scheme {
            self.write(b" ")?;
            self.write(field_name.as_bytes())?;
        }
        self.write(b"\n")?;
        for value in &global.values {
            self.encode_value(value, &mut e_id_transform)?;
        }
        Ok(())
    }
}
