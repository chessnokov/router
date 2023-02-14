use async_trait::async_trait;

pub trait Consumer<'a> {
    type Request;
    fn consume(&mut self, message: Self::Request);
}

#[async_trait]
pub trait Producer {
    type Response<'a>
    where
        Self: 'a;
    async fn produce(&mut self) -> Self::Response<'_>;
}

pub trait Service<'request>: Consumer<'request> + Producer {}
impl<'request, T> Service<'request> for T where T: Consumer<'request> + Producer {}
