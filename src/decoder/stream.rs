use std::{
    cell::UnsafeCell,
    error::Error as ErrorExt,
    fmt::{self, Display},
    io::{Error as IoError, ErrorKind as IoErrorKind},
};

use tokio::io::{AsyncRead, AsyncReadExt};

use super::{BufDecoder, Error, FromBytes};

#[allow(async_fn_in_trait)]
pub trait AsyncDecoder {
    async fn decode<'a, T>(&'a mut self) -> Result<T, StreamError<<T as FromBytes<'a>>::Error>>
    where
        T: FromBytes<'a>;
}

#[derive(Debug)]
pub struct BufStreamDecoder<R> {
    inner: UnsafeCell<BufDecoder>,
    reader: R,
}

impl<R> BufStreamDecoder<R> {
    pub fn new(reader: R, size: usize) -> Self {
        Self {
            inner: UnsafeCell::new(BufDecoder::new(size)),
            reader,
        }
    }
}

#[derive(Debug)]
pub enum StreamError<T> {
    Read(IoError),
    Decode(T),
    OutOfMemory,
}

impl<T: Display> fmt::Display for StreamError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamError::Read(err) => write!(f, "{err}"),
            StreamError::Decode(err) => write!(f, "{err}"),
            StreamError::OutOfMemory => f.write_str("Not enough memory"),
        }
    }
}

impl<T> From<IoError> for StreamError<T> {
    fn from(err: IoError) -> Self {
        StreamError::Read(err)
    }
}

impl<T: fmt::Display + fmt::Debug> ErrorExt for StreamError<T> {}

impl<R> AsyncDecoder for BufStreamDecoder<R>
where
    R: AsyncRead + Unpin,
{
    async fn decode<'a, T>(&'a mut self) -> Result<T, StreamError<<T as FromBytes<'a>>::Error>>
    where
        T: FromBytes<'a>,
    {
        loop {
            // SAFETY: end of borrow when return
            match unsafe { (&mut *self.inner.get()).decode() } {
                Ok(msg) => {
                    return Ok(msg);
                }
                Err(Error::Decode(err)) => return Err(StreamError::Decode(err)),
                Err(Error::Incomplete(n)) => {
                    let needed = n.as_option().unwrap_or(1);
                    let decoder = self.inner.get_mut();
                    let tail = decoder.tail();
                    let free = decoder.free();
                    if free >= needed {
                        if tail < needed {
                            decoder.shift();
                        }
                        let n = self.reader.read(decoder.to_write()).await?;
                        if n > 0 {
                            decoder.write += n;
                        } else {
                            return Err(StreamError::Read(IoError::from(
                                IoErrorKind::UnexpectedEof,
                            )));
                        }
                    } else {
                        return Err(StreamError::OutOfMemory);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::repeat;

    use crate::decoder::Incomplete;

    use super::*;

    #[tokio::test]
    async fn stream_decoder() {
        const MESSAGE_SIZE: usize = 283;

        struct Message<'a>(&'a [u8]);
        impl<'a> FromBytes<'a> for Message<'a> {
            type Error = ();

            fn from_bytes(input: &'a [u8]) -> Result<(&'a [u8], Self), Error<Self::Error>> {
                if let Some((data, tail)) = input.split_at_checked(MESSAGE_SIZE) {
                    Ok((tail, Message(data)))
                } else {
                    Err(Error::Incomplete(Incomplete::Bytes(
                        MESSAGE_SIZE - input.len(),
                    )))
                }
            }
        }

        let mut decoder = BufStreamDecoder::new(repeat(0xAA), 337);

        for _ in 0..1024 * 2 {
            let msg = decoder.decode::<Message>().await.unwrap();
            assert_eq!(msg.0, vec![0xAA; MESSAGE_SIZE]);
        }
    }
}
