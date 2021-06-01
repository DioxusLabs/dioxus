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

static App: FC<()> = |ctx| {
    use_init_recoil_root(ctx, |_| {});
    let todos = use_read(ctx, &TODOS);

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

static Child: FC<ChildProps> = |ctx| {
    let todo = use_read(ctx, &TODOS).get(&ctx.id).unwrap();
    // let (todo, set_todo) = use_read_write(ctx, &TODOS);

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
