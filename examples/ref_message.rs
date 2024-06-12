use std::io::{Error, Seek, Write};

use sick::{
    decoder::{Error as DecodeError, FromBytes},
    encoder::ToBytes,
};

#[derive(Debug)]
#[allow(dead_code)]
pub struct Message<'a>(&'a [u8]);

impl<'bytes> FromBytes<'bytes> for Message<'bytes> {
    type Error = ();
    fn from_bytes(input: &'bytes [u8]) -> Result<(&'bytes [u8], Self), DecodeError<Self::Error>> {
        let (message, tail) = input.split_at(input.len());
        Ok((tail, Self(message)))
    }
}

impl<'a> ToBytes for Message<'a> {
    fn to_bytes<W: Write + Seek>(&self, writer: &mut W) -> Result<usize, Error> {
        writer.write_all(self.0)?;
        Ok(self.0.len())
    }
}

fn main() {}
