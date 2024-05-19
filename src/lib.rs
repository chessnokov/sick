pub use std::io::Error;

use tokio::io::AsyncRead;

pub mod decoder;

pub trait AsyncEncoder<T> {
    async fn encode(&mut self, message: &T) -> Result<(), Error>;
}

pub trait Handle<'a> {
    type Request: 'a;
    type Encoder: AsyncEncoder<Self::Request>;
    async fn consume(
        &mut self,
        request: Self::Request,
        encoder: &mut Self::Encoder,
    ) -> Result<(), Error>;

    async fn produce(&mut self, encoder: &mut Self::Encoder) -> Result<(), Error>;
}

pub struct Service<D, H, E> {
    decoder: D,
    handle: H,
    encoder: E,
}

impl<D, H, E> Service<D, H, E> where D: AsyncRead {}
