use super::ClientRes;
use crate::error::{FromServerFnError, IntoAppError, ServerFnErrorErr};
use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use reqwest::Response;

impl<E: FromServerFnError> ClientRes<E> for Response {
    async fn try_into_string(self) -> Result<String, E> {
        self.text().await.map_err(|e| {
            ServerFnErrorErr::Deserialization(e.to_string()).into_app_error()
        })
    }

    async fn try_into_bytes(self) -> Result<Bytes, E> {
        self.bytes().await.map_err(|e| {
            ServerFnErrorErr::Deserialization(e.to_string()).into_app_error()
        })
    }

    fn try_into_stream(
        self,
    ) -> Result<impl Stream<Item = Result<Bytes, Bytes>> + Send + 'static, E>
    {
        Ok(self.bytes_stream().map_err(|e| {
            E::from_server_fn_error(ServerFnErrorErr::Response(e.to_string()))
                .ser()
        }))
    }

    fn status(&self) -> u16 {
        self.status().as_u16()
    }

    fn status_text(&self) -> String {
        self.status().to_string()
    }

    fn location(&self) -> String {
        self.headers()
            .get("Location")
            .map(|value| String::from_utf8_lossy(value.as_bytes()).to_string())
            .unwrap_or_else(|| self.url().to_string())
    }

    fn has_redirect(&self) -> bool {
        self.headers().get("Location").is_some()
    }
}
