use std::{collections::HashMap, rc::Rc};

use dioxus_core::prelude::*;
use recoil::*;
use uuid::Uuid;

const TODOS: AtomHashMap<Uuid, Rc<Todo>> = |map| {};

#[derive(PartialEq)]
struct Todo {
    checked: bool,
    title: String,
    content: String,
}

static App: FC<()> = |ctx| {
    use_init_recoil_root(ctx, move |cfg| {});

    let todos = use_read_family(ctx, &TODOS);

    // rsx! { in ctx,
    //     div {
    //         h1 {"Basic Todolist with AtomFamilies in Recoil.rs"}
    //         ul { { todos.keys().map(|id| rsx! { Child { id: *id } }) } }
    //     }
    // }
    ctx.render(html! {
        <a href="#" class="">
            <img class="inline-block h-10 w-10 rounded-full object-cover ring-2 ring-white" src="/images/person/4.jpg" alt="Jade"/>
        </a>
    })
};

#[derive(Props, PartialEq)]
struct ChildProps {
    id: Uuid,
}

static Child: FC<ChildProps> = |ctx| {
    let (todo, set_todo) = use_read_write(ctx, &TODOS.select(&ctx.id));

    rsx! { in ctx,
        li {
            h1 {"{todo.title}"}
            input { type: "checkbox", name: "scales", checked: "{todo.checked}" }
            label { "{todo.content}", for: "scales" }
            p {"{todo.content}"}
        }
    }
};

fn main() {
    wasm_bindgen_futures::spawn_local(dioxus_web::WebsysRenderer::start(App))
}
