use async_trait::async_trait;

pub trait Consumer {
    type Request<'a>;

    fn consume(&self, message: Self::Request<'_>);
}

#[async_trait]
pub trait Producer {
    type Response<'a>
    where
        Self: 'a;
    async fn produce(&self) -> Self::Response<'_>;
}

pub trait Service: Consumer + Producer {}

impl<T> Service for T where T: Consumer + Producer {}
