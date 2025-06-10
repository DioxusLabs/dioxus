use dioxus::prelude::*;
use worker::{Context, Env, HttpRequest, event};
use wasm_bindgen::prelude::*;

// https://github.com/rustwasm/wasm-bindgen/issues/4446#issuecomment-2729543167
#[cfg(target_family = "wasm")]
mod wasm_workaround {
    extern "C" {
        pub(super) fn __wasm_call_ctors();
    }
}

// https://github.com/rustwasm/wasm-bindgen/issues/4446#issuecomment-2729543167
#[wasm_bindgen(start)]
fn start() {

    // fix:
   // freestyle::block::_::__ctor::h5e2299a836106c67:: Read a negative address value from the stack. Did we run out of memory?
    #[cfg(target_family = "wasm")]
    unsafe { wasm_workaround::__wasm_call_ctors()};
}

#[cfg(not(feature = "web"))]
#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    _env: Env,
    _ctx: Context,
) -> worker::Result<axum::http::Response<axum::body::Body>> {
    use axum::Router;
    use tower_service::Service;
    console_error_panic_hook::set_once();
    let mut router =
        Router::new().serve_api_application(ServeConfig::builder().build().unwrap(), app);
    Ok(router.call(req).await?)
}

#[server]
async fn hello_world() -> Result<String, ServerFnError> {
    Ok("Hello, world!".to_string())
}

pub fn app() -> Element {
    let hello_world = use_server_future(hello_world)?;
    let hello_world = hello_world().unwrap().unwrap();
    rsx! {
        "{hello_world}"
    }
}
