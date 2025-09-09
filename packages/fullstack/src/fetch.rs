use crate::ServerFnError;

pub fn fetch(url: &str) -> RequestBuilder {
    RequestBuilder::new(url)
}

pub struct RequestBuilder {}

impl RequestBuilder {
    pub fn new(_url: &str) -> Self {
        Self {}
    }

    pub fn method(&mut self, _method: &str) -> &mut Self {
        self
    }

    pub fn json(&mut self, _json: &serde_json::Value) -> &mut Self {
        self
    }

    pub async fn send(&self) -> Result<Response, ServerFnError> {
        Ok(Response {})
    }
}

pub struct Response {}
impl Response {
    pub async fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, ServerFnError> {
        Err(ServerFnError::Serialization("Not implemented".into()))
    }
}
