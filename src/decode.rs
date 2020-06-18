use std::io::{self, Read};
use std::iter::Peekable;

#[derive(Debug)]
pub enum Error {
    Unexpected {
        ex: &'static str,
        got: &'static str,
    },
    BadValueByte(u8),
    Io(io::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

pub struct State<R: Read> {
    idx: usize,
    bytes: Peekable<io::Bytes<R>>,
}

macro_rules! declare_decode_primitive {
    ($name:ident, $t:ty, $desc:literal, $($vars:ident)*) => {
        pub fn $name(&mut self) -> Result<$t, Error> {
            $(
                let $vars = self.next($desc)?;
            )*
            Ok(<$t>::from_be_bytes([$($vars),*]))
        }
    }
}

impl<R: Read> State<R> {
    pub fn new(reader: R) -> Self {
        Self {
            idx: 0,
            bytes: reader.bytes().peekable(),
        }
    }

    pub fn try_next(&mut self) -> Result<Option<u8>, Error> {
        let byte = self.bytes.next().transpose()?;
        if byte.is_some() {
            self.idx += 1;
        }
        Ok(byte)
    }

    pub fn next(&mut self, ex: &'static str) -> Result<u8, Error> {
        match self.try_next()? {
            Some(byte) => Ok(byte),
            None => Err(Error::Unexpected { ex, got: "EOF" }),
        }
    }

    declare_decode_primitive!(decode_u8, u8, "8-bit uint", a);
    declare_decode_primitive!(decode_i8, i8, "8-bit int", a);

    declare_decode_primitive!(decode_u16, u16, "16-bit uint", a b);
    declare_decode_primitive!(decode_i16, i16, "16-bit int", a b);

    declare_decode_primitive!(decode_u32, u32, "32-bit uint", a b c d);
    declare_decode_primitive!(decode_i32, i32, "32-bit int", a b c d);

    declare_decode_primitive!(decode_i64, i64, "64-bit int", a b c d e f g h);

    declare_decode_primitive!(decode_f32, f32, "float", a b c d);
    declare_decode_primitive!(decode_f64, f64, "double", a b c d e f g h);
}
