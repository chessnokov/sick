use core::slice;
use std::{
    error::Error as ErrorExt,
    fmt::{self, Display},
    io::{Error as IoError, ErrorKind as IoErrorKind},
};

use tokio::io::{AsyncRead, AsyncReadExt};

use super::{BufDecoder, Error, FromBytes};

#[allow(async_fn_in_trait)]
pub trait StreamDecoder {
    async fn decode<'a, T>(&'a mut self) -> Result<T, StreamError<<T as FromBytes<'a>>::Error>>
    where
        T: FromBytes<'a>;
}

pub struct BufStreamDecoder<R> {
    inner: BufDecoder,
    reader: R,
}

impl<R> BufStreamDecoder<R> {
    pub fn new(reader: R, size: usize) -> Self {
        Self {
            inner: BufDecoder::new(size),
            reader,
        }
    }
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

impl<T: fmt::Display + fmt::Debug> ErrorExt for StreamError<T> {}

impl<R> StreamDecoder for BufStreamDecoder<R>
where
    R: AsyncRead + Unpin,
{
    async fn decode<'a, T>(&'a mut self) -> Result<T, StreamError<<T as FromBytes<'a>>::Error>>
    where
        T: FromBytes<'a>,
    {
        loop {
            match T::from_bytes(&self.inner.buffer[self.inner.read..self.inner.write]) {
                Ok((tail, msg)) => {
                    self.inner.read = self.inner.write - tail.len();
                    return Ok(msg);
                }
                Err(Error::Decode(err)) => return Err(StreamError::Decode(err)),
                Err(Error::Incomplete(n)) => {
                    let needed = n.as_option().unwrap_or(1);
                    let tail = self.inner.buffer.len() - self.inner.write;
                    let free = tail + self.inner.read;
                    if free >= needed {
                        // Fix borrow checker false error
                        // Safety: because buffer not reallocate or drop
                        let temp = unsafe {
                            slice::from_raw_parts_mut(
                                self.inner.buffer.as_ptr() as *mut u8,
                                self.inner.buffer.len(),
                            )
                        };

                        if tail < needed {
                            temp.copy_within(self.inner.read..self.inner.write, 0);
                            self.inner.write -= self.inner.read;
                            self.inner.read = 0;
                        }

                        let n = self.reader.read(&mut temp[self.inner.write..]).await?;
                        if n > 0 {
                            self.inner.write += n;
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
