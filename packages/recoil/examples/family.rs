use std::collections::HashMap;

use dioxus_core::prelude::*;
use recoil::*;

const TODOS: AtomFamily<&str, Todo> = |_| HashMap::new();

#[derive(PartialEq)]
struct Todo {
    checked: bool,
    contents: String,
}

static App: FC<()> = |ctx, _| {
    rsx! { in ctx,
        div {
            "Basic Todolist with AtomFamilies in Recoil.rs"
        }
    }
};

static Child: FC<()> = |ctx, _| {
    // let todo = use_recoil_value(ctx, &TODOS);
    rsx! { in ctx,
        div {

        }
    }
};

fn main() {
    wasm_bindgen_futures::spawn_local(dioxus_web::WebsysRenderer::start(App))
}
