use super::*;

pub use axum::extract::Form;

impl<T> IntoRequest for Form<T>
where
    T: Serialize + 'static + DeserializeOwned,
{
    fn into_request(self, req: ClientRequest) -> impl Future<Output = ClientResult> + 'static {
        async move { req.send_form(&self.0).await }
    }
}
