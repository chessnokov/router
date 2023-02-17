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
    let mut stream = Stream::new(stream, EchoDecoder, 1024);
    let service = MyService;

    select! {
        Ok(message) = stream.async_decode() => {
            print!("{message:?}");
            service.consume(message);
        }
        message = service.produce() => {
            print!("{message:?}");
        }
    }

    Ok(())
}

pub async fn run<R, D, S, M>(mut stream: Stream<R, D>, service: S)
where
    R: AsyncReadExt + Unpin,
    D: for<'a> Decoder<Item<'a> = M> + 'static,
    S: for<'a> Service<Request<'a> = M, Response<'a> = M> + 'static,
    M: fmt::Debug,
{
    select! {
        Ok(message) = stream.async_decode() => {
            print!("{message:?}");
            service.consume(message);
        }
        message = service.produce() => {
            print!("{message:?}");
        }
    }
}

struct EchoDecoder;
impl Decoder for EchoDecoder {
    type Item<'a> = &'a [u8];
    type Error = ();
    fn decode<'b>(
        &mut self,
        bytes: &'b [u8],
    ) -> Result<Item<'b, Self::Item<'b>>, Error<Self::Error>> {
        Ok(bytes.split_at(bytes.len()))
    }
}

struct MyService;
impl Consumer for MyService {
    type Request<'a> = &'a [u8];

    fn consume(&self, message: Self::Request<'_>) {
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
    async fn produce(&self) -> Self::Response<'_> {
        tokio::time::sleep(Duration::from_secs(1)).await;
        b"Hello World!"
    }
}
