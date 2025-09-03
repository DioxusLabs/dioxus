use server_fn_macro_default::server;
use server_fn::error::ServerFnError;

type PartAlias<T> = Result<T, ServerFnError>;

#[server]
pub async fn part_alias_result() -> PartAlias<String> {
    Ok("hello".to_string())
}

fn main() {}