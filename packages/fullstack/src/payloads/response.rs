use super::*;

impl FromResponse for axum::response::Response {
    fn from_response(res: ClientResponse) -> impl Future<Output = Result<Self, ServerFnError>> {
        async move { todo!() }
    }
}

impl IntoRequest for axum::extract::Request {
    fn into_request(self, request: ClientRequest) -> impl Future<Output = ClientResult> + 'static {
        async move { todo!() }
    }
}
