use std::{error::Error as ErrorExt, fmt};

pub mod stream;

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

#[derive(Debug)]
pub enum Error<T> {
    Incomplete(Option<usize>),
    Custom(T),
}

impl<T: fmt::Display> fmt::Display for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incomplete(None) => f.write_str("Bytes incomplete"),
            Self::Incomplete(Some(bytes)) => write!(f, "Incomplete {bytes}-bytes"),
            Self::Custom(err) => write!(f, "{err}"),
        }
    }
}

impl<T: ErrorExt> ErrorExt for Error<T> {}
