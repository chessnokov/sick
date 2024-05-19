pub use std::io::Error;

use tokio::io::AsyncRead;

pub mod decoder;
pub mod encoder;
#[allow(async_fn_in_trait)]
pub trait AsyncEncoder<T> {
    async fn encode(&mut self, message: &T) -> Result<(), Error>;
}

#[allow(async_fn_in_trait)]
pub trait Handle<'request> {
    type Request;
    type Encoder: AsyncEncoder<Self::Request>;
    async fn consume(
        &mut self,
        request: Self::Request,
        encoder: &mut Self::Encoder,
    ) -> Result<(), Error>;

    async fn produce(&mut self, encoder: &mut Self::Encoder) -> Result<(), Error>;
}

#[allow(dead_code)]
pub struct Service<D, H, E> {
    decoder: D,
    handle: H,
    encoder: E,
}

impl<D, H, E> Service<D, H, E> where D: AsyncRead {}
