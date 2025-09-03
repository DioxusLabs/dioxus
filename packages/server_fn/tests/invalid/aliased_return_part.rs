use server_fn_macro_default::server;

#[derive(Debug, thiserror::Error, Clone, serde::Serialize, serde::Deserialize)]
pub enum InvalidError {
    #[error("error a")]
    A,
}

type PartAlias<T> = Result<T, InvalidError>;

#[server]
pub async fn part_alias_result() -> PartAlias<String> {
    Ok("hello".to_string())
}

fn main() {}