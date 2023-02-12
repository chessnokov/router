use std::{error::Error as ErrorExt, fmt, io};

use tokio::io::AsyncReadExt;

pub type Item<'a, T> = (T, &'a [u8]);
pub trait Decoder<'bytes> {
    type Item;
    type Error;

    /// Item decode
    fn decode(
        &mut self,
        bytes: &'bytes [u8],
    ) -> Result<Item<'bytes, Self::Item>, Error<Self::Error>>;

    /// Fast way to check buffer
    fn check(&mut self, bytes: &'bytes [u8]) -> Result<(), Error<Self::Error>> {
        self.decode(bytes)?;
        Ok(())
    }

    /// Fast way to check needed bytes
    fn is_needed(&mut self, bytes: &'bytes [u8]) -> bool {
        matches!(self.check(bytes), Err(Error::Incomplete(_)))
    }
}

pub enum Error<T> {
    Incomplete(Option<usize>),
    Custom(T),
}

pub struct SplitDecoder<const N: usize>;
impl<'bytes, const N: usize> Decoder<'bytes> for SplitDecoder<N> {
    type Item = &'bytes [u8];
    type Error = ();

    fn decode(
        &mut self,
        bytes: &'bytes [u8],
    ) -> Result<Item<'bytes, Self::Item>, Error<Self::Error>> {
        if bytes.len() < N {
            Err(Error::Incomplete(Some(N - bytes.len())))
        } else {
            Ok(bytes.split_at(N))
        }
    }
}

pub struct ArraySplitDecoder<const N: usize>;
impl<'bytes, const N: usize> Decoder<'bytes> for ArraySplitDecoder<N> {
    type Item = [u8; N];
    type Error = ();

    fn decode(
        &mut self,
        bytes: &'bytes [u8],
    ) -> Result<Item<'bytes, Self::Item>, Error<Self::Error>> {
        if let Some(slice) = bytes.get(..N) {
            Ok((Self::Item::try_from(slice).unwrap(), &bytes[N..]))
        } else {
            Err(Error::Incomplete(Some(N - bytes.len())))
        }
    }
}
#[derive(Debug)]
pub enum StreamError<E> {
    Io(io::Error),
    Decode(E),
    Overflow,
}

impl<E: fmt::Display> fmt::Display for StreamError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{err}"),
            Self::Decode(err) => write!(f, "{err}"),
            Self::Overflow => f.write_str("Buffer overflow"),
        }
    }
}

impl<E: ErrorExt> ErrorExt for StreamError<E> {}

impl<E> From<io::Error> for StreamError<E> {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

pub struct StreamDecoder<R, D> {
    reader: R,
    decoder: D,
    buffer: Vec<u8>,
    read: usize,
    write: usize,
}

impl<R, D> StreamDecoder<R, D> {
    pub fn new(reader: R, decoder: D, size: usize) -> Self {
        Self {
            reader,
            decoder,
            buffer: vec![0; size],
            read: 0,
            write: 0,
        }
    }

    fn shift(&mut self) {
        self.buffer.copy_within(self.read..self.write, 0);
        self.write -= self.read;
        self.read = 0;
    }

    fn tail(&self) -> usize {
        self.buffer.capacity() - self.write
    }

    fn free(&self) -> usize {
        self.tail() + self.read
    }
}

impl<R, D> StreamDecoder<R, D>
where
    R: AsyncReadExt + Unpin,
    D: for<'bytes> Decoder<'bytes>,
{
    pub async fn async_decode(
        &mut self,
    ) -> Result<<D as Decoder<'_>>::Item, StreamError<<D as Decoder<'_>>::Error>> {
        while self.decoder.is_needed(&self.buffer[self.read..self.write]) {
            if self.free() > 0 {
                if self.tail() == 0 {
                    self.shift();
                }
            } else {
                return Err(StreamError::Overflow);
            }

            let n = self.reader.read(&mut self.buffer[self.write..]).await?;
            if n == 0 {
                return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "").into());
            }
            self.write += n;
        }

        match self.decoder.decode(&self.buffer[self.read..self.write]) {
            Ok((item, tail)) => {
                self.read = self.write - tail.len();
                Ok(item)
            }
            Err(Error::Custom(err)) => Err(StreamError::Decode(err)),
            Err(Error::Incomplete(_)) => {
                unreachable!()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[tokio::test]
    async fn stream_decoder() {
        const TAKE: usize = 2;

        let source: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0];

        let mut decoder =
            StreamDecoder::new(Cursor::new(source.clone()), SplitDecoder::<TAKE>, 1024);

        let mut chunks = source.chunks_exact(TAKE);

        loop {
            if let Some(chunk) = chunks.next() {
                assert_eq!(decoder.async_decode().await.unwrap(), chunk);
            } else {
                assert!(matches!(
                    decoder.async_decode().await,
                    Err(StreamError::Io(_))
                ));
                break;
            }
        }
    }
}
