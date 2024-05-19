use std::{
    fmt,
    io::{Error as IoError, Seek, Write},
};

pub trait ToBytes {
    type Error;
    fn to_bytes<W: Write + Seek>(&self, writer: &mut W) -> Result<usize, Error<Self::Error>>;
}

#[derive(Debug)]
pub enum Error<T> {
    Writer(IoError),
    Encode(T),
}

impl<T: fmt::Display> fmt::Display for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Writer(err) => write!(f, "{err}"),
            Error::Encode(err) => write!(f, "{err}"),
        }
    }
}

impl<T> From<IoError> for Error<T> {
    fn from(err: IoError) -> Self {
        Error::Writer(err)
    }
}

pub mod stream {
    use std::io::{Cursor, Seek, SeekFrom};

    use tokio::io::{AsyncWrite, AsyncWriteExt};

    use super::{Error, ToBytes};

    pub struct Encoder<W> {
        buffer: Cursor<Vec<u8>>,
        writer: W,
    }

    impl<W> Encoder<W> {
        pub fn new(writer: W) -> Encoder<W> {
            Encoder {
                buffer: Cursor::new(Vec::new()),
                writer,
            }
        }
    }

    impl<W: AsyncWrite + Unpin> Encoder<W> {
        pub async fn encode<T: ToBytes>(&mut self, message: &T) -> Result<usize, Error<T::Error>> {
            self.buffer.seek(SeekFrom::Start(0))?;
            message.to_bytes(&mut self.buffer)?;
            let pos = self.buffer.position() as usize;
            self.writer.write_all(&self.buffer.get_ref()[..pos]).await?;
            Ok(pos)
        }
    }
}
