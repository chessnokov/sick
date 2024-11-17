use std::io::{Error, Seek, Write};

use anyhow::Error as AnyError;
use sick::{
    decoder::{
        stream::{AsyncDecoder, BufStreamDecoder},
        Error as DecodeError, FromBytes, Incomplete,
    },
    encoder::{stream::BufEncoder, AsyncEncoder, ToBytes},
    AsyncService,
};
use tokio::{net::TcpListener, spawn};

#[derive(Debug)]
pub struct Message(Vec<u8>);

impl<'bytes> FromBytes<'bytes> for Message {
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

impl ToBytes for Message {
    fn to_bytes<W: Write + Seek>(&self, writer: &mut W) -> Result<usize, Error> {
        writer.write_all(self.0.as_slice())?;
        Ok(self.0.len())
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct EchoService;
// TODO: Make possible
// if let Err(err) = EchoService
//     .run(BufStreamDecoder::new(read, 1024), BufEncoder::new(write))
//     .await
// {
//     eprint!("{err}");
// }
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
            let msg = decoder
                .decode::<Message>()
                .await
                .map_err(|err| AnyError::msg(format!("{err}")))?;

            encoder
                .encode(msg)
                .await
                .map_err(|err| AnyError::msg(format!("{err}")))?;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let socket = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Socket bind");
    let (stream, remote) = socket.accept().await?;
    println!("Connection accept from {remote}");
    let (read, write) = stream.into_split();
    let handle = spawn(async move {
        let mut decoder = BufStreamDecoder::new(read, 1024);
        let mut encoder = BufEncoder::new(write);
        loop {
            let msg = match decoder.decode::<Message>().await {
                Ok(msg) => msg,
                Err(err) => {
                    eprintln!("{err}");
                    break;
                }
            };

            if let Err(err) = encoder.encode(msg).await {
                eprintln!("{err}");
                break;
            }
        }
    });
    handle.await?;
    Ok(())
}
