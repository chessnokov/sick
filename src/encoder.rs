use std::io::{Error, Seek, Write};

pub trait ToBytes {
    fn to_bytes<W: Write + Seek>(&self, writer: &mut W) -> Result<usize, Error>;
}

#[allow(async_fn_in_trait)]
pub trait AsyncEncoder {
    async fn encode<M: ToBytes>(&mut self, message: &M) -> Result<usize, Error>;
}

pub mod stream {
    use std::io::{Cursor, Seek, SeekFrom};

    use tokio::io::{AsyncWrite, AsyncWriteExt};

    use super::{AsyncEncoder, Error, ToBytes};

    pub struct BufEncoder<W> {
        buffer: Cursor<Vec<u8>>,
        writer: W,
    }

    impl<W> BufEncoder<W> {
        pub fn new(writer: W) -> BufEncoder<W> {
            BufEncoder {
                buffer: Cursor::new(Vec::new()),
                writer,
            }
        }
    }

    impl<W: AsyncWrite + Unpin> AsyncEncoder for BufEncoder<W> {
        async fn encode<T: ToBytes>(&mut self, message: &T) -> Result<usize, Error> {
            self.buffer.seek(SeekFrom::Start(0))?;
            message.to_bytes(&mut self.buffer)?;
            let pos = self.buffer.position() as usize;
            self.writer.write_all(&self.buffer.get_ref()[..pos]).await?;
            Ok(pos)
        }
    }
}
