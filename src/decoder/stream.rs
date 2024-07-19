use core::slice;
use std::{
    fmt::{self, Display},
    io::{Error as IoError, ErrorKind as IoErrorKind},
};

use tokio::io::{AsyncRead, AsyncReadExt};

use super::{BufDecoder, Error, FromBytes};

#[allow(async_fn_in_trait)]
pub trait StreamDecoder {
    async fn decode<'a, T, R>(
        &'a mut self,
        reader: &mut R,
    ) -> Result<T, StreamError<<T as FromBytes<'a>>::Error>>
    where
        T: FromBytes<'a>,
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

impl StreamDecoder for BufDecoder {
    async fn decode<'a, T, R>(
        &'a mut self,
        reader: &mut R,
    ) -> Result<T, StreamError<<T as FromBytes<'a>>::Error>>
    where
        R: AsyncRead + Unpin,
        T: FromBytes<'a>,
    {
        loop {
            match T::from_bytes(&self.buffer[self.read..self.write]) {
                Ok((tail, msg)) => {
                    self.read = self.write - tail.len();
                    return Ok(msg);
                }
                Err(Error::Decode(err)) => return Err(StreamError::Decode(err)),
                Err(Error::Incomplete(n)) => {
                    let needed = n.as_option().unwrap_or(1);
                    let tail = self.buffer.len() - self.write;
                    let free = tail + self.read;
                    if free >= needed {
                        // Fix borrow checker false error
                        // Safety: because buffer not reallocate or drop
                        let temp = unsafe {
                            slice::from_raw_parts_mut(
                                self.buffer.as_ptr() as *mut u8,
                                self.buffer.len(),
                            )
                        };

                        if tail < needed {
                            temp.copy_within(self.read..self.write, 0);
                            self.write -= self.read;
                            self.read = 0;
                        }

                        let n = reader.read(&mut temp[self.write..]).await?;
                        if n > 0 {
                            self.write += n;
                        } else {
                            return Err(StreamError::Read(IoError::from(
                                IoErrorKind::UnexpectedEof,
                            )));
                        }
                    } else {
                        return Err(StreamError::BufferOverflow);
                    }
                }
            }
        }
    }
}
