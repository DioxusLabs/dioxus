use std::collections::HashMap;

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

#[derive(Default)]
struct CoolData {
    data: HashMap<usize, String>,
}

impl CoolData {
    pub fn new(data: HashMap<usize, String>) -> Self {
        Self { data }
    }

    pub fn view(&self, id: &usize) -> Option<&String> {
        self.data.get(id)
    }

    pub fn set(&mut self, id: usize, data: String) {
        self.data.insert(id, data);
    }
}

#[component]
#[rustfmt::skip]
pub fn App(cx: Scope) -> Element {
    use_shared_state_provider(cx, || CoolData::new(HashMap::from([
        (0, "Hello, World!".to_string()),
        (1, "Dioxus is amazing!".to_string())
    ])));

    render!(
        DataEditor {
            id: 0
        }
        DataEditor {
            id: 1
        }
        DataView {
            id: 0
        }
        DataView {
            id: 1
        }
    )
}

#[component]
fn DataEditor(cx: Scope, id: usize) -> Element {
    let data = use_shared_state::<CoolData>(cx)?;

    render! {
        p {
            {data.read().view(id)?}
        }
    }
}

#[component]
fn DataView(cx: Scope, id: usize) -> Element {
    let data = use_shared_state::<CoolData>(cx)?;

    render! {
        input {
            oninput: move |e: FormEvent| data.write().set(*id, e.value()),
            value: data.read().view(id)?
        }
    }
}
