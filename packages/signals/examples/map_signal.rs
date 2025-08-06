#![allow(non_snake_case)]

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut vec = use_signal(|| vec![0]);

    rsx! {
        button {
            onclick: move |_| {
                let mut write = vec.write();
                let len = write.len() as i32;
                write.push(len);
            },
            "Create"
        }

        button {
            onclick: move |_| {
                vec.write().pop();
            },
            "Destroy"
        }

        for i in 0..vec.len() {
            Child { count: vec.map_mut(move |v| &v[i], move |v| &mut v[i]) }
        }
    }
}

#[component]
fn Child(count: WriteSignal<i32>) -> Element {
    rsx! {
        div {
            "{count}"
        }
    }
}
