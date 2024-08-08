use std::{
    borrow::Cow,
    io::{Error, Seek, Write},
};

use anyhow::Error as AnyError;
use sick::{
    decoder::{
        stream::{AsyncDecoder, BufStreamDecoder},
        Error as DecodeError, FromBytes, Incomplete,
    },
    encoder::{stream::BufEncoder, AsyncEncoder, ToBytes},
    AsyncService,
};
use tokio::net::TcpListener;

#[derive(Debug)]
pub struct Message<'a>(Cow<'a, [u8]>);

impl<'bytes> FromBytes<'bytes> for Message<'bytes> {
    type Error = AnyError;
    fn from_bytes(input: &'bytes [u8]) -> Result<(&'bytes [u8], Self), DecodeError<Self::Error>> {
        if input.is_empty() {
            Err(DecodeError::Incomplete(Incomplete::Bytes(1)))
        } else {
            println!("Decode bytes: {input:02X?}");
            let (message, tail) = input.split_at(1);
            Ok((tail, Self(message.into())))
        }
    }
}

impl<'a> ToBytes for Message<'a> {
    fn to_bytes<W: Write + Seek>(&self, writer: &mut W) -> Result<usize, Error> {
        writer.write_all(self.0.as_ref())?;
        Ok(self.0.len())
    }
}

#[derive(Debug, Copy, Clone, Default)]
struct EchoService;
impl AsyncService for EchoService {
    type Error = AnyError;

    async fn run<D, E>(&mut self, decoder: D, encoder: E) -> Result<(), Self::Error>
    where
        D: AsyncDecoder,
        E: AsyncEncoder,
    {
        let mut encoder = encoder;
        let mut decoder = decoder;
        loop {
            let msg = decoder.decode::<Message>().await?;
            encoder.encode(&msg).await?;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let socket = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Socket bind");
    let (mut stream, remote) = socket.accept().await?;
    println!("Connection accept from {remote}");
    let (read, write) = stream.split();
    EchoService::default()
        .run(BufStreamDecoder::new(read, 1024), BufEncoder::new(write))
        .await?;

    Ok(())
}
