use std::{error::Error as ErrorExt, fmt};

pub mod stream;

pub trait FromBytes<'bytes>: Sized {
    type Error;
    fn from_bytes(input: &'bytes [u8]) -> Result<(&'bytes [u8], Self), Error<Self::Error>>;
}

#[derive(Debug)]
pub enum Error<T> {
    Incomplete(Incomplete),
    Decode(T),
}

impl<T: fmt::Display> fmt::Display for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incomplete(Incomplete::Unknown) => f.write_str("Incomplete data"),
            Self::Incomplete(Incomplete::Bytes(n)) => write!(f, "Incomplete {n}-bytes"),
            Self::Decode(err) => write!(f, "{err}"),
        }
    }
}

impl<T: ErrorExt> ErrorExt for Error<T> {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Incomplete {
    Unknown,
    Bytes(usize),
}

impl Incomplete {
    pub fn as_option(self) -> Option<usize> {
        match self {
            Incomplete::Unknown => None,
            Incomplete::Bytes(n) => Some(n),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BufDecoder {
    buffer: Vec<u8>,
    read: usize,
    write: usize,
}

impl BufDecoder {
    pub fn new(size: usize) -> Self {
        Self {
            buffer: vec![0; size],
            read: 0,
            write: 0,
        }
    }

    pub fn decode<'a, T>(&'a mut self) -> Result<T, Error<T::Error>>
    where
        T: FromBytes<'a>,
    {
        let (tail, value) = T::from_bytes(&self.buffer[self.read..self.write])?;
        self.read = self.write - tail.len();
        Ok(value)
    }

    pub fn tail(&self) -> usize {
        self.buffer.len() - self.write
    }

    pub fn free(&self) -> usize {
        self.read + self.tail()
    }

    pub fn shift(&mut self) {
        if self.read > 0 {
            self.buffer.copy_within(self.read..self.write, 0);
            self.write -= self.read;
            self.read = 0;
        }
    }

    pub fn extend(&mut self, n: usize) {
        self.buffer.resize(self.buffer.len() + n, 0);
    }

    pub fn truncate(&mut self) {
        self.buffer.resize(self.write, 0);
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.len() == 0
    }

    pub fn to_write(&mut self) -> &mut [u8] {
        &mut self.buffer[self.write..]
    }
}
