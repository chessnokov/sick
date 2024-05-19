use sick::decoder::{Error, FromBytes};

#[derive(Debug)]
#[allow(dead_code)]
pub struct Message<'a>(&'a [u8]);

impl<'bytes> FromBytes<'bytes> for Message<'bytes> {
    type Error = ();
    fn from_bytes(input: &'bytes [u8]) -> Result<(&'bytes [u8], Self), Error<Self::Error>> {
        let (message, tail) = input.split_at(input.len());
        Ok((tail, Self(message)))
    }
}

fn main() {}
