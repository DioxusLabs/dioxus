use crate::FromResponse;

pub struct Text<T>(pub T);

impl<T: Into<String>> axum::response::IntoResponse for Text<T> {
    fn into_response(self) -> axum::response::Response {
        axum::response::Response::builder()
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(axum::body::Body::from(self.0.into()))
            .unwrap()
    }
}

impl<T: Into<String>> FromResponse<()> for Text<T> {
    fn from_response(
        res: reqwest::Response,
    ) -> impl std::prelude::rust_2024::Future<
        Output = Result<Self, dioxus_fullstack_core::ServerFnError>,
    > + Send {
        async move {
            let status = res.status();
            // let text = res
            //     .text()
            //     .await
            //     .map_err(dioxus_fullstack_core::ServerFnError::Reqwest)?;
            // if !status.is_success() {
            //     return Err(dioxus_fullstack_core::ServerFnError::StatusCode(status));
            // }
            // Ok(Text(text))
            todo!()
        }
    }
}
