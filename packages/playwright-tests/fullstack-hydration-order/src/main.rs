// Adjacent server components that both use server functions shouldn't cause
// hydration issues
// https://github.com/DioxusLabs/dioxus/issues/4595

use dioxus::prelude::*;

#[server(ServerStrings)]
pub async fn get_strings() -> Result<Vec<String>, ServerFnError> {
    let data: Vec<String> = vec!["Hello".to_string(), "World".to_string()];
    Ok(data)
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

#[server(ServerFloats)]
pub async fn get_floats() -> Result<Vec<f32>, ServerFnError> {
    let data: Vec<f32> = vec![1.0, 2.0, 3.0];
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
    }
}
