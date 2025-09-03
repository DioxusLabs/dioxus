use server_fn_macro_default::server;

#[derive(Debug, thiserror::Error, Clone, serde::Serialize, serde::Deserialize)]
pub enum InvalidError {
    #[error("error a")]
    A,
}

type FullAlias = Result<String, InvalidError>;

#[server]
pub async fn full_alias_result() -> FullAlias {
    Ok("hello".to_string())
}

fn main() {}