use std::io;

pub use super::decode::Error as DecodeError;

#[derive(Debug)]
pub enum Error {
    Decode(usize, DecodeError),
    Encode(io::Error),
}
