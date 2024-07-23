use std::{
    borrow::Cow,
    io::{Error, Seek, Write},
    os::fd::AsRawFd,
};

use anyhow::Error as AnyError;
use sick::{
    decoder::{BufDecoder, Error as DecodeError, FromBytes, Incomplete},
    encoder::{stream::BufEncoder, ToBytes},
    make_service, Handle,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::{channel, Receiver, Sender},
};

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

struct EchoService {
    tx: Sender<Vec<u8>>,
    rx: Receiver<Vec<u8>>,
}

impl Default for EchoService {
    fn default() -> Self {
        let (tx, rx) = channel(1);
        Self { tx, rx }
    }
}

impl<'request> Handle<'request> for EchoService {
    type Request = Message<'request>;

    async fn call(&mut self, request: Self::Request) {
        let bytes = request.0.into_owned();
        println!("Recieve request: {bytes:?}");
        self.tx.send(bytes).await.unwrap()
    }

    async fn poll<E: sick::encoder::Encoder>(&mut self, encoder: &mut E) -> Result<(), Error> {
        let bytes = self.rx.recv().await.unwrap();
        println!("Send response: {bytes:?}");
        encoder.encode(&Message(bytes.into())).await.map(|_| ())
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
    make_service(
        EchoService::default(),
        BufDecoder::new(1024),
        BufEncoder::new(writer),
        reader,
    )
    .await?;

    Ok(())
}
