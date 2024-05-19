use std::{io::Error as IoError, marker::PhantomData};

use tokio::io::{AsyncRead, AsyncReadExt};

use super::{Error, FromBytes};

pub enum StreamError<T> {
    Read(IoError),
    Decode(T),
    BufferOverflow,
}

impl<T> From<IoError> for StreamError<T> {
    fn from(err: IoError) -> Self {
        StreamError::Read(err)
    }
}

pub struct StreamDecoder<R, T> {
    reader: R,
    buffer: Vec<u8>,
    read: usize,
    write: usize,
    _item: PhantomData<T>,
}

impl<R, T> StreamDecoder<R, T>
where
    R: AsyncRead + Unpin,
    T: for<'a> FromBytes<'a>,
{
    pub async fn async_decode(&mut self) -> Result<T, StreamError<<T as FromBytes<'_>>::Error>> {
        let Self {
            ref mut reader,
            ref mut buffer,
            ref mut read,
            ref mut write,
            _item,
        } = *self;
        while let Some(n) = T::incomplited(&buffer[*read..*write]) {
            let needed = n.amount();
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
