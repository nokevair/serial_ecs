use std::io;

pub use super::decode::Error as DecodeError;

#[derive(Debug)]
pub enum Error {
    Decode(DecodeError),
    Encode(io::Error),
}
