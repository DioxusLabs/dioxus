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

type PartAlias<T> = Result<T, CustomError>;

#[server]
pub async fn part_alias_result() -> PartAlias<String> {
    Ok("hello".to_string())
}

fn main() {}
