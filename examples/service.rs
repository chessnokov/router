use std::{fmt, time::Duration};

use async_trait::async_trait;
use tokio::{io::AsyncReadExt, main, net::TcpListener, select};

use router::{
    decoder::{stream::Stream, Decoder, Error, Item},
    service::*,
};

#[main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    let (stream, addr) = listener.accept().await?;
    println!("Connect from {addr}");
    Server::run(Stream::new(stream, EchoDecoder, 1024), MyService).await;

    Ok(())
}

struct EchoDecoder;
impl<'bytes> Decoder<'bytes> for EchoDecoder {
    type Item = &'bytes [u8];
    type Error = ();
    fn decode(
        &mut self,
        bytes: &'bytes [u8],
    ) -> Result<Item<'bytes, Self::Item>, Error<Self::Error>> {
        Ok(bytes.split_at(bytes.len()))
    }
}

struct MyService;
impl<'a> Consumer<'a> for MyService {
    type Request = &'a [u8];
    fn consume(&mut self, message: Self::Request) {
        if let Ok(s) = std::str::from_utf8(message) {
            println!("Receive: {s}");
        } else {
            println!("Receive: {message:X?}");
        }
    }
}

#[async_trait]
impl Producer for MyService {
    type Response<'a> = &'a [u8];
    async fn produce(&mut self) -> Self::Response<'_> {
        tokio::time::sleep(Duration::from_secs(1)).await;
        b"Hello World!"
    }
}

struct Server;
impl Server {
    async fn run<R, D, S, M>(mut decoder: Stream<R, D>, mut service: S)
    where
        R: AsyncReadExt + Unpin,
        D: for<'a> Decoder<'a, Item = M>,
        M: fmt::Debug,
        S: for<'a> Service<'a, Request = M, Response<'a> = M> + 'static,
    {
        select! {
            Ok(message) = decoder.async_decode() => {
                print!("{message:?}");
                service.consume(message);
            }
            message = service.produce() => {
                print!("{message:?}");
            }
        }
    }
}
