use std::{error::Error as ErrorExt, fmt, io};

use tokio::io::AsyncReadExt;

pub type Item<'a, T> = (T, &'a [u8]);
pub trait Decoder<'bytes> {
    type Item;
    type Error;
    fn decode(
        &mut self,
        bytes: &'bytes [u8],
    ) -> Result<Item<'bytes, Self::Item>, Error<Self::Error>>;
    fn is_needed(&mut self, bytes: &'bytes [u8]) -> bool {
        matches!(self.decode(bytes), Err(Error::Incomplete(_)))
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
        let Self {
            ref mut reader,
            ref mut decoder,
            ref mut buffer,
            ref mut read,
            ref mut write,
        } = *self;

        while decoder.is_needed(&buffer[*read..*write]) {
            let tail = buffer.capacity() - *write;
            let free = tail + *read;
            if free > 0 {
                if tail == 0 {
                    buffer.copy_within(*read..*write, 0);
                    *write -= *read;
                    *read = 0;
                }
            } else {
                return Err(StreamError::Overflow);
            }

            let n = reader.read(&mut buffer[*write..]).await?;
            if n == 0 {
                return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "").into());
            }
            *write += n;
        }

        match decoder.decode(&buffer[*read..*write]) {
            Ok((item, tail)) => {
                *read = *write - tail.len();
                Ok(item)
            }
            Err(Error::Custom(err)) => Err(StreamError::Decode(err)),
            Err(Error::Incomplete(_)) => {
                unreachable!()
            }
        }
    }
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
