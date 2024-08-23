use std::{io::Write, net::ToSocketAddrs, time::Instant};

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

fn async_message_bench<'a, const N: usize>(bencher: &mut Bencher<'a>) {
    const BUFFER_MESSAGES: usize = 64;

    let runtime = Runtime::new().unwrap();
    bencher.to_async(&runtime).iter_custom(|iters| {
        let socket = TcpSocket::new_v4().unwrap();
        socket
            .bind("127.0.0.1:30000".to_socket_addrs().unwrap().next().unwrap())
            .unwrap();
        let buffer_size = socket.send_buffer_size().unwrap();

        let listener = socket.listen(1024).unwrap();
        let data = vec![0xFF_u8; buffer_size as usize];

        runtime.spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            while socket.write_all(data.as_slice()).await.is_ok() {}
        });

        async move {
            let stream = TcpStream::connect("127.0.0.1:30000").await.unwrap();
            let mut decoder = BufStreamDecoder::new(stream, N * BUFFER_MESSAGES);
            let start = Instant::now();
            for _i in 0..iters {
                black_box(decoder.decode::<Message<N>>().await).unwrap();
            }
            start.elapsed()
        }
    })
}

fn semi_async_message_bench<'a, const N: usize>(bencher: &mut Bencher<'a>) {
    const BUFFER_MESSAGES: usize = 64;

    let runtime = Runtime::new().unwrap();
    bencher.to_async(&runtime).iter_custom(|iters| {
        let listener = std::net::TcpListener::bind("127.0.0.1:30000").unwrap();
        // cat /proc/sys/net/ipv4/tcp_wmem
        let data = vec![0xFF_u8; 4194304];

        runtime.spawn_blocking(move || {
            let (mut socket, _) = listener.accept().unwrap();
            while socket.write_all(data.as_slice()).is_ok() {}
        });

        async move {
            let stream = TcpStream::connect("127.0.0.1:30000").await.unwrap();
            let mut decoder = BufStreamDecoder::new(stream, N * BUFFER_MESSAGES);
            let start = Instant::now();
            for _i in 0..iters {
                black_box(decoder.decode::<Message<N>>().await).unwrap();
            }
            start.elapsed()
        }
    })
}

fn benchmark(c: &mut Criterion) {
    c.bench_function("async decode 1k", async_message_bench::<1024>);
    c.bench_function("semi async decode 1k", semi_async_message_bench::<1024>);
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
