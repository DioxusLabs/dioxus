use server_fn::{
    codec::JsonEncoding,
    error::{FromServerFnError, ServerFnErrorErr},
};
use server_fn_macro_default::server;

#[derive(
    Debug, thiserror::Error, Clone, serde::Serialize, serde::Deserialize,
)]
pub enum CustomError {
    #[error("error a")]
    ErrorA,
    #[error("error b")]
    ErrorB,
}

impl FromServerFnError for CustomError {
    type Encoder = JsonEncoding;

    fn from_server_fn_error(_: ServerFnErrorErr) -> Self {
        Self::ErrorA
    }
}

#[server]
pub async fn no_alias_result() -> Result<String, CustomError> {
    Ok("hello".to_string())
}

fn main() {}
