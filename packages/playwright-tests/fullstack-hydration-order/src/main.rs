// Adjacent server components that both use server functions shouldn't cause
// hydration issues
// https://github.com/DioxusLabs/dioxus/issues/4595

use dioxus::prelude::*;

fn main() {
    dioxus::launch(|| rsx! { Home {} });
}

#[component]
pub fn Home() -> Element {
    let mut count = use_signal(|| 0);
    rsx! {
        div {
            MyStrings {}
            MyFloats {}
        }
        button {
            id: "counter",
            onclick: move |_| count += 1,
            "Count {count}"
        }
        TrailingEmpties {}
    }
}

// Regression test for the markerless hydration walker.
// Two trailing empty dynamic texts after a non-empty one: the SSR HTML has zero
// bytes for the empties, so the merged DOM text node is just the non-empty
// prefix. The walker's `SynthTextAfter` opcode must synthesize the empties in
// document order — if it inserts each new node before `cursor.nextSibling`
// without advancing the cursor, every later insert lands *before* the previous
// one, reversing them. The visible regression only surfaces when the dynamic
// texts later become non-empty, since adjacent empty text nodes look identical
// to two empty text nodes in any order.
#[component]
fn TrailingEmpties() -> Element {
    let mut a = use_signal(String::new);
    let mut b = use_signal(String::new);
    rsx! {
        div {
            id: "trailing-empties",
            "FIRST"
            "{a}"
            "{b}"
        }
        button {
            id: "fill-trailing",
            onclick: move |_| {
                a.set("[a]".to_string());
                b.set("[b]".to_string());
            },
            "Fill"
        }
    }
}

#[component]
fn MyStrings() -> Element {
    let strings = use_server_future(get_strings)?;
    let data = match &*strings.read() {
        Some(Ok(data)) => data.clone(),
        _ => vec![],
    };

    rsx! {
        div {
            for string in data.iter() {
                p { "{string}" }
            }
        }
    }
}
#[get("/api/get_strings")]
pub async fn get_strings() -> Result<Vec<String>, ServerFnError> {
    let data: Vec<String> = vec!["Hello".to_string(), "World".to_string()];
    Ok(data)
}

#[component]
fn MyFloats() -> Element {
    let floats = use_server_future(get_floats)?;
    let data = match &*floats.read() {
        Some(Ok(data)) => data.clone(),
        _ => vec![],
    };

    rsx! {
        div {
            for float in data.iter() {
                p { "{float}" }
            }
        }
    }
}

#[get("/api/get_floats")]
pub async fn get_floats() -> Result<Vec<f32>, ServerFnError> {
    let data: Vec<f32> = vec![1.0, 2.0, 3.0];
    Ok(data)
}
