use server_fn_macro_default::server;

#[derive(Debug, thiserror::Error, Clone, serde::Serialize, serde::Deserialize)]
pub enum InvalidError {
    #[error("error a")]
    A,
}

#[server]
pub async fn no_alias_result() -> Result<String, InvalidError> {
    Ok("hello".to_string())
}

fn main() {}