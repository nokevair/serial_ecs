use std::io;

#[derive(Debug)]
pub enum Error {
    Parse {
        ex: &'static str,
        got: &'static str,
    },
    BadValueByte(u8),
    Io(io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}
