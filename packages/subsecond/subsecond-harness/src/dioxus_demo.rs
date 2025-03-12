use dioxus::prelude::*;

pub fn launch() -> anyhow::Result<()> {
    dioxus::launch(app);
    Ok(())
}

fn app() -> Element {
    let mut count = use_signal(|| 0);
    let abcv = 105;

    rsx! {
        h1 { "{count}" }
        button {
            onclick: move |_| {
                count.set(count() + 1);
            },
            "Increment {abcv}"
        }
        button {
            onclick: move |_| {
                count.set(count() + 2);
            },
            "Increment {abcv}"
        }
        button {
            onclick: move |_| {
                count.set(count() + 1);
            },
            "Increment {abcv}"
        }
        div { "hello world" }
        for x in 0..12 {
            Child { id: x, opt: "hello".to_string() }
        }
    }
}

#[component]
fn Child(id: u32, opt: String) -> Element {
    let mut count = use_signal(|| 2);
    rsx! {
        div {
            h3 { "Child: {id} - {opt}" }
            p { "count: {count}" }
            button { onclick: move |_| { count += 1 }, "Increment Count" }
        }
    }
}
#[component]
fn Child4(id: u32, opt: String) -> Element {
    rsx! {
        div { "Hello ?? child: {id} - {opt} ?" }
    }
}

#[component]
fn Child3(id: u32, opt: String) -> Element {
    rsx! {
        div { "Hello ?? child: {id} - {opt} ?" }
    }
}

#[component]
fn Child2(id: u32, opt: String) -> Element {
    rsx! {
        div { "oh lordy!" }
        div { "Hello ?? child2s: {id} - {opt} ?" }
    }
}
