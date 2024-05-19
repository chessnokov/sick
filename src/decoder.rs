use std::io::Error as IoError;

use tokio::io::{AsyncRead, AsyncReadExt};

pub trait FromBytes: Sized {
    type Error;
    fn from_bytes<'a>(input: &'a [u8]) -> Result<(&'a [u8], Self), Error<Self::Error>>;
}

pub trait Decoder {
    type Item: FromBytes;
    fn decode<'a>(
        &mut self,
        input: &'a [u8],
    ) -> Result<(&'a [u8], Self::Item), Error<<Self::Item as FromBytes>::Error>> {
        Self::Item::from_bytes(input)
    }
}

pub struct StreamDecoder<R, D> {
    reader: R,
    decoder: D,
    buffer: Vec<u8>,
    read: usize,
    write: usize,
}

impl<R, D> StreamDecoder<R, D> {
    pub fn new(capacity: usize, reader: R, decoder: D) -> StreamDecoder<R, D> {
        StreamDecoder {
            reader,
            decoder,
            buffer: vec![0; capacity],
            read: 0,
            write: 0,
        }
    }
}

impl<R, D> StreamDecoder<R, D>
where
    R: AsyncRead + Unpin,
    D: Decoder,
{
    pub async fn async_decode(
        &mut self,
    ) -> Result<D::Item, StreamError<<D::Item as FromBytes>::Error>> {
        let Self {
            ref mut reader,
            ref mut buffer,
            ref mut read,
            ref mut write,
            ref mut decoder,
        } = *self;
        while let Err(Error::Incomplete(n)) = decoder.decode(&buffer[*read..*write]) {
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

        match decoder.decode(&buffer[*read..*write]) {
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
