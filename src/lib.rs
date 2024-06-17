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
pub struct Service<R, H, E> {
    reader: R,
    decoder: BufDecoder,
    handle: H,
    encoder: E,
}

impl<R, H, E> Service<R, H, E> {
    pub fn new(reader: R, decoder: BufDecoder, handle: H, encoder: E) -> Service<R, H, E> {
        Service {
            reader,
            decoder,
            handle,
            encoder,
        }
    }
}

impl<R, H, E, T> Service<R, H, E>
where
    R: AsyncRead + Unpin,
    T: for<'a> FromBytes<'a>,
    for<'a> <T as FromBytes<'a>>::Error: Display,
    H: for<'a> Handle<'a, Request = T>,
    E: Encoder,
{
    pub async fn handle(&mut self) {
        loop {
            select! {
                request = self.decoder.decode(&mut self.reader) => {
                    match request {
                        Ok(request) => {
                            self.handle.call(request).await;
                        }
                        Err(err) => {
                            error!("{err}");
                            break;
                        },
                    }
                }
                result = self.handle.poll(&mut self.encoder) => {
                    if let Err(err) = result {
                        error!("{err}");
                        break;
                    }
                }
            }
        }
    }
}
