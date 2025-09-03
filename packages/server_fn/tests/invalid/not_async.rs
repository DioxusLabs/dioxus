use server_fn_macro_default::server;
use server_fn::error::ServerFnError;

#[server]
pub fn not_async() -> Result<String, ServerFnError> {
    Ok("hello".to_string())
}

fn main() {}