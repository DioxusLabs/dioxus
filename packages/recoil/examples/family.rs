use std::collections::HashMap;

use dioxus_core::prelude::*;
use recoil::*;
use uuid::Uuid;

const TODOS: AtomFamily<Uuid, Todo> = |_| HashMap::new();

#[derive(PartialEq)]
struct Todo {
    checked: bool,
    title: String,
    contents: String,
}

static App: FC<()> = |ctx, _| {
    use_init_recoil_root(ctx);

    let todos = use_recoil_family(ctx, &TODOS);

    rsx! { in ctx,
        div {
            "Basic Todolist with AtomFamilies in Recoil.rs"
        }
    }
};

#[derive(Props, PartialEq)]
struct ChildProps {
    id: Uuid,
}

static Child: FC<ChildProps> = |ctx, props| {
    let (todo, set_todo) = use_recoil_state(ctx, &TODOS.select(&props.id));

    rsx! { in ctx,
        div {
            h1 {"{todo.title}"}
            input { type: "checkbox", name: "scales", checked: "{todo.checked}" }
            label { "{todo.contents}", for: "scales" }
            p {"{todo.contents}"}
        }
    }
};

fn main() {
    wasm_bindgen_futures::spawn_local(dioxus_web::WebsysRenderer::start(App))
}
