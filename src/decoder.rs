pub mod stream;

pub trait FromBytes<'bytes>: Sized {
    type Error;
    fn from_bytes(input: &'bytes [u8]) -> Result<(&'bytes [u8], Self), Error<Self::Error>>;
    /// Implement this only if exist method to fast check input filled
    fn incomplited(input: &'bytes [u8]) -> Option<Incomplete> {
        if let Err(Error::Incomplete(n)) = Self::from_bytes(input) {
            Some(n)
        } else {
            None
        }
    }
}

pub enum Error<T> {
    Incomplete(Incomplete),
    Decode(T),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Incomplete {
    Unknown,
    Bytes(usize),
}

impl Incomplete {
    pub fn amount(self) -> usize {
        match self {
            Incomplete::Unknown => 0,
            Incomplete::Bytes(n) => n,
        }
    }
}
