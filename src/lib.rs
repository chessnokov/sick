pub use std::io::Error as IoError;

use decoder::stream::StreamDecoder;
use encoder::Encoder;

pub mod decoder;
pub mod encoder;

/// Abstract Stateful service,
/// this will replace `Handle`
#[allow(async_fn_in_trait)]
pub trait Service {
    type Error;
    async fn run<D, E>(&mut self, decoder: D, encoder: E) -> Result<(), Self::Error>
    where
        D: StreamDecoder,
        E: Encoder;
}
