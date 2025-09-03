use server_fn_macro_default::server;

#[server]
pub async fn empty_return() -> () {
    ()
}

fn main() {}