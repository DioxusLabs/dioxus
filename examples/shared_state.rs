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
    let cool_data = use_shared_state::<CoolData>(cx).unwrap().read();

    let my_data = &cool_data.view(id).unwrap();

    render!(p {
        "{my_data}"
    })
}

#[component]
fn DataView(cx: Scope, id: usize) -> Element {
    let cool_data = use_shared_state::<CoolData>(cx).unwrap();

    let oninput = |e: FormEvent| cool_data.write().set(*id, e.value.clone());

    let cool_data = cool_data.read();
    let my_data = &cool_data.view(id).unwrap();

    render!(input {
        oninput: oninput,
        value: "{my_data}"
    })
}
