use std::fmt::Display;
pub use std::io::Error;

use decoder::{stream::StreamDecoder, BufDecoder, FromBytes};
use encoder::Encoder;
use log::error;
use tokio::{io::AsyncRead, select};

pub mod decoder;
pub mod encoder;

#[allow(async_fn_in_trait)]
pub trait Handle<'request> {
    type Request: FromBytes<'request>;
    async fn call(&mut self, request: Self::Request);
    /// Return only std::io::Error from `Encoder`
    async fn poll<E: Encoder>(&mut self, encoder: &mut E) -> Result<(), Error>;
}

#[allow(dead_code)]
pub struct EndPoint<R, E> {
    reader: R,
    decoder: BufDecoder,
    encoder: E,
}

impl<R, E> EndPoint<R, E> {
    pub fn new(reader: R, decoder: BufDecoder, encoder: E) -> EndPoint<R, E> {
        EndPoint {
            reader,
            decoder,
            encoder,
        }
    }
}

impl<R, E> EndPoint<R, E>
where
    R: AsyncRead + Unpin,
    E: Encoder,
{
    pub async fn handle<H, T>(&mut self, handler: &mut H)
    where
        T: for<'a> FromBytes<'a>,
        for<'a> <T as FromBytes<'a>>::Error: Display,
        H: for<'a> Handle<'a, Request = T>,
    {
        loop {
            select! {
                request = self.decoder.decode(&mut self.reader) => {
                    match request {
                        Ok(request) => {
                            handler.call(request).await;
                        }
                        Err(err) => {
                            error!("{err}");
                            break;
                        },
                    }
                }
                result = handler.poll(&mut self.encoder) => {
                    if let Err(err) = result {
                        error!("{err}");
                        break;
                    }
                }
            }
        }
    }
}
