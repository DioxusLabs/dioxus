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
    A,
    #[error("error b")]
    B,
}

impl FromServerFnError for CustomError {
    type Encoder = JsonEncoding;

    fn from_server_fn_error(_: ServerFnErrorErr) -> Self {
        Self::A
    }
}

#[server]
pub async fn full_alias_result() -> CustomError {
    CustomError::A
}

fn main() {}
