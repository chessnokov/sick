use std::{net::ToSocketAddrs, time::Instant};

use anyhow::Error as AnyError;
use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion};
use sick::decoder::{
    stream::{AsyncDecoder, BufStreamDecoder},
    Error, FromBytes, Incomplete,
};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpSocket, TcpStream},
    runtime::Runtime,
    spawn,
};

#[derive(Debug)]
pub struct Message<const N: usize>;

impl<'bytes, const N: usize> FromBytes<'bytes> for Message<N> {
    type Error = AnyError;
    fn from_bytes(input: &'bytes [u8]) -> Result<(&'bytes [u8], Self), Error<Self::Error>> {
        if let Some((_message, tail)) = input.split_at_checked(N) {
            Ok((tail, Self))
        } else {
            Err(Error::Incomplete(Incomplete::Bytes(N - input.len())))
        }
    }
}

fn message_bench<'a, const N: usize>(bencher: &mut Bencher<'a>) {
    const BUFFER_MESSAGES: usize = 64;

    let runtime = Runtime::new().unwrap();
    let _guard = runtime.enter();
    bencher.to_async(&runtime).iter_custom(|iters| async move {
        let socket = TcpSocket::new_v4().unwrap();
        socket
            .bind("127.0.0.1:30000".to_socket_addrs().unwrap().next().unwrap())
            .unwrap();
        let buffer_size = socket.send_buffer_size().unwrap();

        let listener = socket.listen(1024).unwrap();
        let data = vec![0xFF_u8; buffer_size as usize];

        spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            while socket.write_all(data.as_slice()).await.is_ok() {}
        });

        let stream = TcpStream::connect("127.0.0.1:30000").await.unwrap();
        let mut decoder = BufStreamDecoder::new(stream, N * BUFFER_MESSAGES);
        let start = Instant::now();
        for _i in 0..iters {
            black_box(decoder.decode::<Message<N>>().await).unwrap();
        }
        start.elapsed()
    })
}

fn benchmark(c: &mut Criterion) {
    c.bench_function("decode 1k", message_bench::<1024>);
    c.bench_function("decode 2k", message_bench::<{ 1024 * 2 }>);
    c.bench_function("decode 4k", message_bench::<{ 1024 * 4 }>);
    c.bench_function("decode 8k", message_bench::<{ 1024 * 8 }>);
    c.bench_function("decode 16k", message_bench::<{ 1024 * 16 }>);
    c.bench_function("decode 32k", message_bench::<{ 1024 * 32 }>);
    c.bench_function("decode 64k", message_bench::<{ 1024 * 64 }>);
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
