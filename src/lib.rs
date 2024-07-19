use std::future::Future;
pub use std::io::Error;

use decoder::{stream::StreamDecoder, FromBytes};
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

pub fn make_service<H, D, E, R>(
    handler: H,
    decoder: D,
    encoder: E,
    reader: R,
) -> impl Future<Output = ()>
where
    // T: FromBytes<'a>,
    // <T as FromBytes<'a>>::Error: Display,
    H: for<'a> Handle<'a>,
    R: AsyncRead + Unpin,
    E: Encoder,
    D: StreamDecoder,
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
                        Err(_err) => {
                            // error!("{err}");
                            break;
                        },
                    }
                }
                result = handler.poll(&mut encoder) => {
                    if let Err(err) = result {
                        error!("{err}");
                        break;
                    }
                }
            }
        }
    }
}
