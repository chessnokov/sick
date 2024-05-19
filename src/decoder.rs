use std::fmt::{self, Display};

pub mod stream;

pub trait FromBytes<'bytes>: Sized {
    type Error;
    fn from_bytes(input: &'bytes [u8]) -> Result<(&'bytes [u8], Self), Error<Self::Error>>;
    /// Implement this only if exist method to fast check input filled,
    /// for example:
    /// ```
    /// if input.len() < REQUIRED {
    ///   return Some(Incomplete::Bytes(REQUIRED))
    /// }
    fn incomplited(input: &'bytes [u8]) -> Option<Incomplete> {
        if let Err(Error::Incomplete(n)) = Self::from_bytes(input) {
            Some(n)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub enum Error<T> {
    Incomplete(Incomplete),
    Decode(T),
}

impl<T: Display> fmt::Display for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incomplete(Incomplete::Unknown) => f.write_str("Incomplete data"),
            Self::Incomplete(Incomplete::Bytes(n)) => write!(f, "Incomplete {n}-bytes"),
            Self::Decode(err) => write!(f, "{err}"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Incomplete {
    Unknown,
    Bytes(usize),
}

impl Incomplete {
    pub fn around(self) -> usize {
        match self {
            Incomplete::Unknown => 1,
            Incomplete::Bytes(n) => n,
        }
    }
}
