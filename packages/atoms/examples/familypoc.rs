use dioxus_core::prelude::*;
use im_rc::HashMap as ImMap;
use recoil::*;
use uuid::Uuid;

const TODOS: Atom<ImMap<Uuid, Todo>> = |_| ImMap::new();

#[derive(PartialEq)]
struct Todo {
    checked: bool,
    title: String,
    contents: String,
}

static App: FC<()> = |cx| {
    use_init_recoil_root(cx, |_| {});
    let todos = use_read(&cx, &TODOS);

    rsx! { in cx,
        div {
            "Basic Todolist with AtomFamilies in Recoil.rs"
        }
    }
};

#[derive(Props, PartialEq)]
struct ChildProps {
    id: Uuid,
}

static Child: FC<ChildProps> = |cx| {
    let todo = use_read(cx, &TODOS).get(&cx.id).unwrap();
    // let (todo, set_todo) = use_read_write(cx, &TODOS);

    rsx! { in cx,
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
