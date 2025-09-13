use axum::response::IntoResponse;

pub struct ServerSentEvents<T> {
    _t: std::marker::PhantomData<*const T>,
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SseError {}

impl<T> ServerSentEvents<T> {
    pub fn new() -> Self {
        Self {
            _t: std::marker::PhantomData,
        }
    }

    pub async fn next(&mut self) -> Option<Result<T, SseError>> {
        todo!()
    }
}

impl IntoResponse for ServerSentEvents<String> {
    fn into_response(self) -> axum::response::Response {
        todo!()
    }
}
