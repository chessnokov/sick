use std::{
    fmt::{self, Display},
    io::Error as IoError,
};

use tokio::io::{AsyncRead, AsyncReadExt};

use super::{Error, FromBytes};

#[allow(async_fn_in_trait)]
pub trait StreamDecoder {
    async fn decode<T, R>(
        &mut self,
        reader: &mut R,
    ) -> Result<T, StreamError<<T as FromBytes<'_>>::Error>>
    where
        T: for<'a> FromBytes<'a>,
        R: AsyncRead + Unpin;
}

#[derive(Debug)]
pub enum StreamError<T> {
    Read(IoError),
    Decode(T),
    BufferOverflow,
}

impl<T: Display> fmt::Display for StreamError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamError::Read(err) => write!(f, "{err}"),
            StreamError::Decode(err) => write!(f, "{err}"),
            StreamError::BufferOverflow => f.write_str("Not enough memory"),
        }
    }
}

impl<T> From<IoError> for StreamError<T> {
    fn from(err: IoError) -> Self {
        StreamError::Read(err)
    }
}

pub struct RingBuf {
    buffer: Vec<u8>,
    read: usize,
    write: usize,
}

impl RingBuf {
    pub async fn decode<R, T>(
        &mut self,
        reader: &mut R,
    ) -> Result<T, StreamError<<T as FromBytes<'_>>::Error>>
    where
        R: AsyncRead + Unpin,
        T: for<'a> FromBytes<'a>,
    {
        let Self {
            ref mut buffer,
            ref mut read,
            ref mut write,
        } = *self;
        while let Some(n) = T::incomplited(&buffer[*read..*write]) {
            let needed = n.around();
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
                    let n = reader.read(&mut buffer[*write..]).await?;
                    *write += n;
                    total += n;
                } else {
                    return Err(StreamError::BufferOverflow);
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
