//! We can use custom types as inputs and outputs to server functions, provided they implement the right traits.

use axum::extract::FromRequest;
use dioxus::prelude::*;
use dioxus_fullstack::{use_websocket, Websocket};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut message = use_signal(|| "Send a message!".to_string());

    // if ws.connecting() {
    //     return rsx! { "Connecting..." };
    // }

    rsx! {
        input {
            oninput: move |e| async move {
                // _ = ws.send(()).await;
            },
            placeholder: "Type a message",
        }
    }
}

// struct MyInputStream {}
// impl<S> FromRequest<S> for MyInputStream {
//     #[doc = " If the extractor fails it\'ll use this \"rejection\" type. A rejection is"]
//     #[doc = " a kind of error that can be converted into a response."]
//     type Rejection = ();

//     #[doc = " Perform the extraction."]
//     fn from_request(
//         req: dioxus_server::axum::extract::Request,
//         state: &S,
//     ) -> impl std::prelude::rust_2024::Future<Output = Result<Self, Self::Rejection>> + Send {
//         async move { todo!() }
//     }
// }

// impl dioxus_fullstack::IntoRequest for MyInputStream {
//     type Input = ();

//     type Output = ();

//     fn into_request(input: Self::Input) -> std::result::Result<Self::Output, ServerFnError> {
//         todo!()
//     }
// }
