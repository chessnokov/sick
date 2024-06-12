use std::fmt::Display;
pub use std::io::Error;

use decoder::{stream::Decoder, FromBytes};
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
pub struct Service<R, T, H, E> {
    decoder: Decoder<R, T>,
    handle: H,
    encoder: E,
}

impl<R, T, H, E> Service<R, T, H, E> {
    pub fn new(decoder: Decoder<R, T>, handle: H, encoder: E) -> Service<R, T, H, E> {
        Service {
            decoder,
            handle,
            encoder,
        }
    }
}

impl<R, T, H, E> Service<R, T, H, E>
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
                request = self.decoder.decode() => {
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
