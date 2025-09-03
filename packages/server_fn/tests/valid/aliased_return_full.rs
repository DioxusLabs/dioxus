use server_fn_macro_default::server;
use server_fn::error::ServerFnError;

type FullAlias = Result<String, ServerFnError>;

#[server]
pub async fn full_alias_result() -> FullAlias {
    Ok("hello".to_string())
}

fn main() {}