pub use std::io::Error as IoError;
use std::{fmt::Display, future::Future};

use anyhow::Error as AnyError;
use decoder::{stream::StreamDecoder, FromBytes};
use encoder::Encoder;
use tokio::{io::AsyncRead, select};

pub mod decoder;
pub mod encoder;

#[allow(async_fn_in_trait)]
pub trait Handle<'request> {
    type Request: FromBytes<'request>;
    async fn call(&mut self, request: Self::Request);
    /// Return only std::io::Error from `Encoder`
    async fn poll<E: Encoder>(&mut self, encoder: &mut E) -> Result<(), IoError>;
}

pub fn make_service<H, D, E, R>(
    handler: H,
    decoder: D,
    encoder: E,
    reader: R,
) -> impl Future<Output = Result<(), AnyError>>
where
    H: for<'a> Handle<'a>,
    R: AsyncRead + Unpin,
    E: Encoder,
    D: StreamDecoder,
    for<'a> <<H as Handle<'a>>::Request as FromBytes<'a>>::Error: Display,
{
    async move {
        let mut reader = reader;
        let mut encoder = encoder;
        let mut handler = handler;
        let mut decoder = decoder;
        loop {
            select! {
                request = decoder.decode(&mut reader) => {
                    match request {
                        Ok(request) => {
                            handler.call(request).await;
                        }
                        Err(err) => {
                            return Err(AnyError::msg(format!("{err}")));
                        },
                    }
                }
                result = handler.poll(&mut encoder) => {
                    if let Err(err) = result {
                        return Err(AnyError::msg(format!("{err}")));
                    }
                }
            }
        }
    }
}
