use std::{
    borrow::Cow,
    io::{Error, Seek, Write},
};

use anyhow::Error as AnyError;
use sick::{
    decoder::{BufDecoder, Error as DecodeError, FromBytes},
    encoder::{stream::BufEncoder, ToBytes},
    EndPoint, Handle,
};
use tokio::{
    net::TcpListener,
    sync::mpsc::{channel, Receiver, Sender},
};

#[derive(Debug)]
#[allow(dead_code)]
pub struct Message<'a>(Cow<'a, [u8]>);

impl<'bytes> FromBytes<'bytes> for Message<'bytes> {
    type Error = AnyError;
    fn from_bytes(input: &'bytes [u8]) -> Result<(&'bytes [u8], Self), DecodeError<Self::Error>> {
        let (message, tail) = input.split_at(input.len());
        Ok((tail, Self(message.into())))
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
        self.tx.send(bytes).await.unwrap()
    }

    async fn poll<E: sick::encoder::Encoder>(&mut self, encoder: &mut E) -> Result<(), Error> {
        let bytes = self.rx.recv().await.unwrap();
        encoder.encode(&Message(bytes.into())).await.map(|_| ())
    }
}

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let socket = TcpListener::bind("0.0.0.0:8080").await?;
    let (stream, _remote) = socket.accept().await?;
    let (reader, writer) = stream.into_split();
    let mut endpoint = EndPoint::new(reader, BufDecoder::new(1024), BufEncoder::new(writer));
    let mut handler = EchoService::default();

    endpoint.handle(&mut handler).await;

    Ok(())
}
