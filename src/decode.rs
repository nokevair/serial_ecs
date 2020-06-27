use std::ascii;
use std::borrow::Cow;
use std::io::{self, Read};
use std::iter::Peekable;

#[derive(Debug)]
pub enum Error {
    Unexpected {
        idx: usize,
        ex: Cow<'static, str>,
        got: Cow<'static, str>,
    },
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
    // special case: 24-bit uint
    (u24) => {
        pub fn decode_u24(&mut self) -> Result<u32, Error> {
            Ok(u32::from_be_bytes([
                0,
                self.next("24-bit uint")?,
                self.next("24-bit uint")?,
                self.next("24-bit uint")?,
            ]))
        }
    };
    
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

    pub fn err_unexpected(
        &self,
        ex: impl Into<Cow<'static, str>>,
        got: impl Into<Cow<'static, str>>,
    ) -> Error {
        Error::Unexpected {
            idx: self.idx,
            ex: ex.into(),
            got: got.into(),
        }
    }

    pub fn try_next(&mut self) -> Result<Option<u8>, Error> {
        let byte = self.bytes.next().transpose()?;
        if byte.is_some() {
            self.idx += 1;
        }
        Ok(byte)
    }

    pub fn next(&mut self, ex: impl Into<Cow<'static, str>>) -> Result<u8, Error> {
        match self.try_next()? {
            Some(byte) => Ok(byte),
            None => Err(self.err_unexpected(ex, "EOF")),
        }
    }

    pub fn expect_newline(&mut self) -> Result<(), Error> {
        let byte = self.next("newline")?;
        if byte == b'\n' {
            Ok(())
        } else {
            Err(self.err_unexpected(
                "newline",
                format!("non-newline byte: {}", ascii::escape_default(byte)),
            ))
        }
    }

    declare_decode_primitive!(decode_u8, u8, "8-bit uint", a);
    declare_decode_primitive!(decode_i8, i8, "8-bit int", a);

    declare_decode_primitive!(decode_u16, u16, "16-bit uint", a b);
    declare_decode_primitive!(decode_i16, i16, "16-bit int", a b);

    declare_decode_primitive!(u24);

    declare_decode_primitive!(decode_u32, u32, "32-bit uint", a b c d);
    declare_decode_primitive!(decode_i32, i32, "32-bit int", a b c d);

    declare_decode_primitive!(decode_i64, i64, "64-bit int", a b c d e f g h);

    declare_decode_primitive!(decode_f32, f32, "float", a b c d);
    declare_decode_primitive!(decode_f64, f64, "double", a b c d e f g h);

    pub fn decode_header_line(&mut self, ex: &'static str) -> Result<Vec<String>, Error> {
        let mut line = String::new();
        loop {
            let byte = self.next(ex)?;
            if byte == b'\n' {
                break;
            } else if byte.is_ascii() {
                line.push(byte as char);
            } else {
                return Err(self.err_unexpected(
                    ex,
                    format!("non-ASCII byte: {}", ascii::escape_default(byte)),
                ))
            }
        }
        Ok(line.split_whitespace().map(String::from).collect())
    }
}
