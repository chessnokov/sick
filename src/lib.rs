use std::fmt::Display;
pub use std::io::Error;

use decoder::{stream::Decoder, FromBytes};
use encoder::stream::Encoder;
use log::error;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    select,
};

pub mod decoder;
pub mod encoder;

#[allow(async_fn_in_trait)]
pub trait Handle<'request> {
    type Request: FromBytes<'request>;
    async fn consume(&mut self, request: Self::Request);

    /// Return only `Encoder` Error
    async fn produce<W: AsyncWrite + Unpin>(
        &mut self,
        encoder: &mut Encoder<W>,
    ) -> Result<Loop, Error>;
}

pub enum Loop {
    /// Continue working
    Continue,
    /// Break loop and close socket
    Break,
}

#[allow(dead_code)]
pub struct Service<R, T, H, W> {
    decoder: Decoder<R, T>,
    handle: H,
    encoder: Encoder<W>,
}

impl<R, T, H, W> Service<R, T, H, W> {
    pub fn new(decoder: Decoder<R, T>, handle: H, encoder: Encoder<W>) -> Service<R, T, H, W> {
        Service {
            decoder,
            handle,
            encoder,
        }
    }
}

impl<R, T, H, W> Service<R, T, H, W>
where
    R: AsyncRead + Unpin,
    T: for<'a> FromBytes<'a>,
    for<'a> <T as FromBytes<'a>>::Error: Display,
    H: for<'a> Handle<'a, Request = T>,
    W: AsyncWrite + Unpin,
{
    pub async fn handle(&mut self) {
        loop {
            select! {
                request = self.decoder.decode() => {
                    match request {
                        Ok(request) => {
                            self.handle.consume(request).await;
                        }
                        Err(err) => {
                            error!("{err}");
                            break;
                        },
                    }
                }
                result = self.handle.produce(&mut self.encoder) => {
                    match result {
                        Ok(Loop::Continue) => {}
                        Ok(Loop::Break) => {
                            break;
                        }
                        Err(err) => {
                            error!("{err}");
                            break;
                        }
                    }
                }
            }
        }
    }
}
