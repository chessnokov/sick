pub use std::io::Error as IoError;

use decoder::stream::AsyncDecoder;
use encoder::AsyncEncoder;

pub mod decoder;
pub mod encoder;

/// Abstract Stateful service,
#[allow(async_fn_in_trait)]
pub trait AsyncService {
    type Error;
    async fn run<D, E>(&mut self, decoder: D, encoder: E) -> Result<(), Self::Error>
    where
        D: AsyncDecoder,
        E: AsyncEncoder;
}
