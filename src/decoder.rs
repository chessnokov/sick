use std::{io::Error as IoError, marker::PhantomData};

use tokio::io::{AsyncRead, AsyncReadExt};

// TODO: try to use frombytes::FromBytes
pub trait FromBytes: Sized {
    type Error;
    fn from_bytes<'a>(input: &'a [u8]) -> Result<(&'a [u8], Self), Error<Self::Error>>;
}

pub struct StreamDecoder<R, T> {
    reader: R,
    buffer: Vec<u8>,
    read: usize,
    write: usize,
    _item: PhantomData<T>,
}

impl<R, T> StreamDecoder<R, T> {
    pub fn new(capacity: usize, reader: R) -> StreamDecoder<R, T> {
        StreamDecoder {
            reader,
            buffer: vec![0; capacity],
            read: 0,
            write: 0,
            _item: PhantomData,
        }
    }
}

impl<R, T> StreamDecoder<R, T>
where
    R: AsyncRead + Unpin,
    T: FromBytes,
{
    pub async fn async_decode(&mut self) -> Result<T, StreamError<T::Error>> {
        let Self {
            ref mut reader,
            ref mut buffer,
            ref mut read,
            ref mut write,
            _item,
        } = *self;
        while let Err(Error::Incomplete(n)) = T::from_bytes(&buffer[*read..*write]) {
            let needed = n.unwrap_or_default();
            let mut total = 0;
            loop {
                let tail = buffer.len() - *write;
                let free = tail + *read;

                if free >= needed {
                    // TODO: Move this to strategy
                    if *read > tail {
                        // Shift data
                        buffer.copy_within(*read..*write, 0);
                        *write -= *read;
                        *read = 0;
                    }
                    let n = reader
                        .read(&mut buffer[*write..])
                        .await
                        .map_err(ReadError::from)?;
                    *write += n;
                    total += n;
                } else {
                    return Err(ReadError::BufferOverflow.into());
                }

                if total >= needed {
                    break;
                }
            }
        }

        match T::from_bytes(&buffer[*read..*write]) {
            Ok((tail, message)) => {
                *read = *write - tail.len();
                return Ok(message);
            }
            Err(Error::Decode(err)) => {
                return Err(StreamError::Decode(err));
            }
            Err(Error::Incomplete(_)) => {
                unreachable!()
            }
        }
    }
}

#[derive(Debug)]
enum ReadError {
    Io(IoError),
    BufferOverflow,
}

impl From<IoError> for ReadError {
    fn from(err: IoError) -> Self {
        Self::Io(err)
    }
}

pub enum StreamError<T> {
    Read(ReadError),
    Decode(T),
}

impl<T> From<ReadError> for StreamError<T> {
    fn from(err: ReadError) -> Self {
        StreamError::Read(err)
    }
}

pub enum Error<T> {
    Incomplete(Option<usize>),
    Decode(T),
}
