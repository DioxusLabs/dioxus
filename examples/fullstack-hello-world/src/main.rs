//! Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // let mut t = use_signal(|| 0);
    rsx! {
        h1 { "Hot patch serverfns!" }
        // button {
        //     onclick: move |_| {
        //         t += 1;
        //     },
        //     "Say hi!"
        // }
        // "{t}"
    }
}

// fn app() -> Element {
//     let mut text = use_signal(|| "...".to_string());

//     rsx! {
//         h1 { "Hot patch serverfns!" }
//         button {
//             onclick: move |_| async move {
//                 text.set(say_hi().await.unwrap());
//             },
//             "Say hi!"
//         }
//         button {
//             onclick: move |_| async move {
//                 text.set("wooo!!!".to_string());
//             },
//             "Say hi!!!!"
//         }
//         button {
//             onclick: move |_| async move {
//                 text.set("wooo2!!!".to_string());
//             },
//             "Say hi!"
//         }
//         button {
//             onclick: move |_| async move {
//                 text.set("wooo3!!!".to_string());
//             },
//             "Say hi!"
//         }
//         "Server said: {text}"
//         Child1 { i: 123 }
//         Child3 { i: "one" }
//         // Child3 { i: "two" }
//         // Child3 { i: "three" }
//         // Child3 { i: "four" }
//         // Child3 { i: "five" }
//         // Child3 { i: "six" }
//         // Child3 { i: "seven" }
//     }
// }

// #[component]
// fn Child1(i: i32) -> Element {
//     let abc = 123;
//     rsx! {
//         div { "Hello from the child component!" }
//     }
// }

// #[component]
// fn Child3(i: String) -> Element {
//     let mut state = use_signal(|| 0);
//     rsx! {
//         div {
//             h3 { "Hello from the child component {i} -> {state}!"  }
//             button {
//                 onclick: move |_| state += 1,
//                 "Increment"
//             }
//         }
//     }
// }

// #[server]
// async fn say_hi() -> Result<String, ServerFnError> {
//     Ok("DUAL achieved!".to_string())
// }

// #[server]
// async fn say_bye() -> Result<String, ServerFnError> {
//     Ok("goodbye!".to_string())
// }

// #[server]
// async fn say_bye2() -> Result<String, ServerFnError> {
//     Ok("goodbye1!".to_string())
// }

// #[server]
// async fn say_bye3() -> Result<String, ServerFnError> {
//     Ok("goodbye2!".to_string())
// }
