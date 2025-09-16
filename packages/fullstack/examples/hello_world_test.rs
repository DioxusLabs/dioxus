use std::num::ParseFloatError;

use dioxus::prelude::*;

fn main() {
    dioxus::launch(|| {
        let mut res = use_signal(|| "Hello World!".to_string());

        let mut fetcher = move |_| async move {
            let res2 = parse_number("123".to_string()).await.unwrap();
            res.set(res2);
        };

        rsx! {
            h1 { "fetch me! {res.read()}" }
            button { onclick: fetcher, "Click me!" }
        }
    });
}

#[post("/api/parse/?number")]
async fn parse_number(number: String) -> Result<String, ServerFnError> {
    let parsed_number: f32 = number
        .parse()
        .map_err(|e: ParseFloatError| ServerFnError::Args(e.to_string()))?;
    Ok(format!("Parsed number: {}", parsed_number))
}
