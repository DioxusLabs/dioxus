use server_fn_macro_default::server;
use server_fn::error::ServerFnError;

#[server]
pub async fn no_alias_result() -> Result<String, ServerFnError> {
    Ok("hello".to_string())
}


fn main() {}