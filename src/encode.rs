use std::fmt::Display;
use std::io::{self, Write};

pub struct State<W> {
    out: W,
}

impl<W: Write> State<W> {
    pub fn new(out: W) -> State<W> {
        Self { out }
    }

    pub fn write(&mut self, buf: &[u8]) -> io::Result<()> {
        self.out.write_all(buf)
    }

    pub fn write_fmt<T: Display>(&mut self, t: T) -> io::Result<()> {
        write!(self.out, "{}", t)
    }
}
