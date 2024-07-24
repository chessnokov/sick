use std::{
    borrow::Cow,
    io::{Error, Seek, Write},
    os::fd::AsRawFd,
};

use anyhow::Error as AnyError;
use sick::{
    decoder::{
        stream::{BufStreamDecoder, StreamDecoder},
        Error as DecodeError, FromBytes, Incomplete,
    },
    encoder::{stream::BufEncoder, Encoder, ToBytes},
    Service,
};
use tokio::net::{TcpListener, TcpStream};

#[derive(Debug)]
#[allow(dead_code)]
pub struct Message<'a>(Cow<'a, [u8]>);

impl<'bytes> FromBytes<'bytes> for Message<'bytes> {
    type Error = AnyError;
    fn from_bytes(input: &'bytes [u8]) -> Result<(&'bytes [u8], Self), DecodeError<Self::Error>> {
        println!("Decode bytes: {input:?}");
        if input.is_empty() {
            Err(DecodeError::Incomplete(Incomplete::Bytes(1)))
        } else {
            let (message, tail) = input.split_at(input.len());
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
impl Service for EchoService {
    type Error = AnyError;

    async fn run<D, E>(&mut self, decoder: D, encoder: E) -> Result<(), Self::Error>
    where
        D: StreamDecoder,
        E: Encoder,
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
    let (stream, remote) = socket.accept().await?;
    println!("Connection accept from {remote}");
    let read = stream.into_std()?;
    let write = read.try_clone()?;
    println!("Read: {}, write: {}", read.as_raw_fd(), write.as_raw_fd());
    let reader = TcpStream::from_std(read)?;
    let writer = TcpStream::from_std(write)?;
    println!("Socket sucessfuly split");
    EchoService::default()
        .run(BufStreamDecoder::new(reader, 1024), BufEncoder::new(writer))
        .await?;

    Ok(())
}
