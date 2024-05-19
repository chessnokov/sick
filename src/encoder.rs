use std::io::{Error as IoError, Seek, Write};

pub trait ToBytes {
    type Error;
    fn to_bytes<W: Write + Seek>(&self, writer: &mut W) -> Result<usize, Error<Self::Error>>;
}

pub enum Error<T> {
    Writer(IoError),
    Encode(T),
}
