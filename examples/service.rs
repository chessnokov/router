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
    let stream = Stream::new(stream, EchoDecoder, 1024);
    run(stream).await;

    Ok(())
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

async fn run<R, D>(mut decoder: Stream<R, D>)
where
    R: AsyncReadExt + Unpin + 'static,
    D: Decoder + 'static,
    D::Item<'static>: fmt::Debug + 'static,
{
    if let Ok(message) = decoder.async_decode().await {
        print!("{message:?}");
    }
}
