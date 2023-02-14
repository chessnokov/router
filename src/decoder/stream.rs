use std::{error::Error as ErrorExt, fmt, io};

use tokio::io::AsyncReadExt;

use super::{Decoder, Error as DecodeError};

#[derive(Debug)]
pub enum Error<E> {
    Io(io::Error),
    Decode(E),
    Overflow,
}

impl<E: fmt::Display> fmt::Display for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{err}"),
            Self::Decode(err) => write!(f, "{err}"),
            Self::Overflow => f.write_str("Buffer overflow"),
        }
    }
}

impl<E: ErrorExt> ErrorExt for Error<E> {}

impl<E> From<io::Error> for Error<E> {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

pub struct Stream<R, D> {
    reader: R,
    decoder: D,
    buffer: Vec<u8>,
    read: usize,
    write: usize,
}

impl<R, D> Stream<R, D> {
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

impl<R, D> Stream<R, D>
where
    R: AsyncReadExt + Unpin,
    D: for<'bytes> Decoder<'bytes>,
{
    pub async fn async_decode(
        &mut self,
    ) -> Result<<D as Decoder<'_>>::Item, Error<<D as Decoder<'_>>::Error>> {
        while self.decoder.is_needed(&self.buffer[self.read..self.write]) {
            if self.free() > 0 {
                if self.tail() == 0 {
                    self.shift();
                }
            } else {
                return Err(Error::Overflow);
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
            Err(DecodeError::Custom(err)) => Err(Error::Decode(err)),
            Err(DecodeError::Incomplete(_)) => {
                unreachable!()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::decoder::Item;

    use super::*;

    const TAKE: usize = 2;

    #[tokio::test]
    async fn decoder_ref() {

        struct SliceSplitDecoder<const N: usize>;
        impl<'bytes, const N: usize> Decoder<'bytes> for SliceSplitDecoder<N> {
            type Item = &'bytes [u8];
            type Error = ();
    
            fn decode(
                &mut self,
                bytes: &'bytes [u8],
            ) -> Result<Item<'bytes, Self::Item>, DecodeError<Self::Error>> {
                if bytes.len() < N {
                    Err(DecodeError::Incomplete(Some(N - bytes.len())))
                } else {
                    Ok(bytes.split_at(N))
                }
            }
        }
    
        let source: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0];

        let mut decoder = Stream::new(Cursor::new(source.clone()), SliceSplitDecoder::<TAKE>, 1024);

        let mut chunks = source.chunks_exact(TAKE);

        loop {
            if let Some(chunk) = chunks.next() {
                assert_eq!(decoder.async_decode().await.unwrap(), chunk);
            } else {
                assert!(matches!(decoder.async_decode().await, Err(Error::Io(_))));
                break;
            }
        }
    }

    #[tokio::test]
    async fn decoder_own() {

        struct ArraySplitDecoder<const N: usize>;
        impl<'bytes, const N: usize> Decoder<'bytes> for ArraySplitDecoder<N> {
            type Item = [u8; N];
            type Error = ();
    
            fn decode(
                &mut self,
                bytes: &'bytes [u8],
            ) -> Result<Item<'bytes, Self::Item>, DecodeError<Self::Error>> {
                if let Some(slice) = bytes.get(..N) {
                    Ok((Self::Item::try_from(slice).unwrap(), &bytes[N..]))
                } else {
                    Err(DecodeError::Incomplete(Some(N - bytes.len())))
                }
            }
        }
    
        let source: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0];

        let mut decoder = Stream::new(Cursor::new(source.clone()), ArraySplitDecoder::<TAKE>, 1024);

        let mut chunks = source.chunks_exact(TAKE);

        loop {
            if let Some(chunk) = chunks.next() {
                assert_eq!(decoder.async_decode().await.unwrap(), chunk);
            } else {
                assert!(matches!(decoder.async_decode().await, Err(Error::Io(_))));
                break;
            }
        }
    }
}
