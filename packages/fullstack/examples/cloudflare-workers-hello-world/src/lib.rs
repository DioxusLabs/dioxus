use std::panic;
use tracing_web::MakeConsoleWriter;
use dioxus::prelude::*;
#[cfg(feature = "server")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "server")]
#[worker::event(start)]
fn start() {
    use tracing_subscriber::prelude::*;

    #[wasm_bindgen]
    extern {
        #[wasm_bindgen(js_namespace = console)]
        fn error(msg: String);

        type Error;

        #[wasm_bindgen(constructor)]
        fn new() -> Error;

        #[wasm_bindgen(structural, method, getter)]
        fn stack(error: &Error) -> String;
    }

    pub fn hook(info: &panic::PanicInfo) {
        let mut msg = info.to_string();
        msg.push_str("\n\nStack:\n\n");

        let e = Error::new();
        let stack = e.stack();
        msg.push_str(&stack);
        msg.push_str("\n\n");

        // let bt = backtrace::Backtrace::new();
        // msg.push_str(&format!("backtrace: {:?}", bt));

        let _ = error(msg);
    }

    panic::set_hook(Box::new(hook));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .without_time()
        .with_writer(MakeConsoleWriter);
    // let perf_layer = tracing_web::performance_layer().with_details_from_fields(tracing_subscriber::fmt::format::Pretty::default());
    tracing_subscriber::registry()
        .with(fmt_layer)
        // .with(perf_layer)
        .init();

    GetServerData::register_explicit().unwrap();
}

#[cfg(feature = "server")]
#[worker::event(fetch)]
async fn main(req: worker::Request, env: worker::Env, ctx: worker::Context) -> worker::Result<worker::Response> {
    let ls = tokio::task::LocalSet::new();
    let guard = ls.enter();
    let handler = serve_dioxus_application("/api/");
    let rep = handler(req, env);
    ls.run_until(rep).await
}

pub fn app() -> Element {
    let mut count = use_signal(|| 0);
    let text = use_signal(|| "...".to_string());
    let server_future = use_server_future(get_server_data)?;

    rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
        // button {
        //     onclick: move |_| {
        //         to_owned![text];
        //         async move {
        //             if let Ok(data) = get_server_data().await {
        //                 println!("Client received: {}", data);
        //                 text.set(data.clone());
        //                 post_server_data(data).await.unwrap();
        //             }
        //         }
        //     },
        //     "Run a server function!"
        // }
        "Server said: {text}"
        // "{server_future.state():?}"
    }
}


#[server("/api")]
async fn get_server_data() -> Result<String, ServerFnError> {
    Ok(reqwest::get("https://httpbin.org/ip").await?.text().await?)
}
