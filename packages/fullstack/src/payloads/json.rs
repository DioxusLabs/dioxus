use super::*;
pub use axum::extract::Json;

impl<T> IntoRequest for Json<T>
where
    T: Serialize + 'static,
{
    fn into_request(self, request: ClientRequest) -> impl Future<Output = ClientResult> + 'static {
        async move { request.send_json(&self.0).await }
    }
}

impl<T: DeserializeOwned> FromResponse for Json<T> {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move {
            let data = res.json::<T>().await?;
            Ok(Json(data))
        }
    }
}
