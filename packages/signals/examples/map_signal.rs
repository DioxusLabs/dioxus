#![allow(non_snake_case)]

use dioxus::prelude::*;

fn main() {
    launch(app);
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
            Child { count: vec.map(move |v| &v[i]) }
        }
    }
}

#[component]
fn Child(count: MappedSignal<i32>) -> Element {
    use_memo({
        to_owned![count];
        move || {
            let value = count.read();
            println!("Child value: {value}");
        }
    });

    rsx! {
        div {
            "Child: {count}"
        }
    }
}
