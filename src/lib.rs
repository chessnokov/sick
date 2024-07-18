pub use std::io::Error;
use std::{fmt::Display, future::Future};

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
pub struct Terminal<R, E> {
    reader: R,
    decoder: BufDecoder,
    encoder: E,
}

impl<R, E> Terminal<R, E> {
    pub fn new(reader: R, decoder: BufDecoder, encoder: E) -> Terminal<R, E> {
        Terminal {
            reader,
            decoder,
            encoder,
        }
    }
}

impl<R, E> Terminal<R, E>
where
    R: AsyncRead + Unpin,
    E: Encoder,
{
    pub async fn handle<'a, H, T>(&'a mut self, _handler: H)
    where
        T: FromBytes<'a>,
        <T as FromBytes<'a>>::Error: Display,
        H: Handle<'a, Request = T>,
    {
        let Self {
            reader: _,
            decoder: _,
            encoder: _,
        } = self;
        loop {
            // select! {
            //     request = decoder.decode(reader) => {
            //         match request {
            //             Ok(request) => {
            //                 handler.call(request).await;
            //             }
            //             Err(err) => {
            //                 error!("{err}");
            //                 break;
            //             },
            //         }
            //     }
            //     result = handler.poll(encoder) => {
            //         if let Err(err) = result {
            //             error!("{err}");
            //             break;
            //         }
            //     }
            // }
        }
    }
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
