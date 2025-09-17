use axum::extract::FromRequest;
use bytes::Bytes;
use http::HeaderMap;

pub struct ServerResponse {
    headers: HeaderMap,
    status: http::StatusCode,
}

impl ServerResponse {
    pub async fn new_from_reqwest(res: reqwest::Response) -> Self {
        let status = res.status();
        let headers = res.headers();
        todo!()
    }
}
