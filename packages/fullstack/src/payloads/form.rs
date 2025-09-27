use super::*;

pub use axum::extract::Form;

impl<T> IntoRequest for Form<T>
where
    T: Serialize + 'static,
{
    fn into_request(self, req: ClientRequest) -> impl Future<Output = ClientResult> + 'static {
        async move {
            // For GET and HEAD requests, we encode the form data as query parameters.
            // For other request methods, we encode the form data as the request body.
            if matches!(*req.method(), Method::GET | Method::HEAD) {
                return req.query(&self.0).send().await;
            }

            let body = serde_urlencoded::to_string(&self.0)
                .map_err(|err| RequestError::Body(err.to_string()))?;

            req.header("Content-Type", "application/x-www-form-urlencoded")
                .body(body)
                .send()
                .await
        }
    }
}
