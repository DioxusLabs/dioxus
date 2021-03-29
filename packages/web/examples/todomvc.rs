use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::AtomicUsize,
};

use dioxus_core::prelude::*;
use dioxus_web::WebsysRenderer;
use uuid::Uuid;

// Entry point
fn main() {
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(|ctx, props| {
        ctx.create_context(|| model::TodoManager::new());

        ctx.render(rsx! {
            div {
                TodoList {}
                Footer {}
            }
        })
    }))
}

static TodoList: FC<()> = |ctx, props| {
    let todos = use_state_new(&ctx, || BTreeMap::<usize, model::TodoItem>::new());

    let items = todos.iter().map(|(order, item)| {
        rsx!(TodoItem {
            // key: "{}",
            todo: item
        })
    });

    ctx.render(rsx! {
        div {
            {items}
        }
    })
};

#[derive(Debug, PartialEq, Props)]
struct TodoItemsProp<'a> {
    todo: &'a model::TodoItem,
}

fn TodoItem(ctx: Context, props: &TodoItemsProp) -> DomTree {
    let (editing, set_editing) = use_state(&ctx, || false);

    let id = props.todo.id;
    ctx.render(rsx! (
        li {
            div {
                "{id}"
            }
            // {input}
        }
    ))
}

static Footer: FC<()> = |ctx, props| {
    ctx.render(html! {
        <footer className="info">
            <p>"Double-click to edit a todo"</p>
            <p>
                "Created by "<a href="http://github.com/jkelleyrtp/">"jkelleyrtp"</a>
            </p>
            <p>
                "Part of "<a href="http://todomvc.com">"TodoMVC"</a>
            </p>
        </footer>
    })
};

// The data model that the todo mvc uses
mod model {
    use std::{borrow::BorrowMut, future::Future};

    use super::*;

    #[derive(Debug, PartialEq, Clone)]
    pub struct TodoItem {
        pub id: Uuid,
        pub checked: bool,
        pub contents: String,
    }

    struct Dispatcher {}

    struct AppContext<T: Clone> {
        _t: std::rc::Rc<T>,
    }

    impl<T: Clone> AppContext<T> {
        fn dispatch(&self, f: impl FnOnce(&mut T)) {}
        fn async_dispatch(&self, f: impl Future<Output = ()>) {}
        fn get<G>(&self, f: impl Fn(&T) -> &G) -> &G {
            f(&self._t)
        }
        fn set(&self, orig: &mut std::borrow::Cow<T>) {
            let r = orig.to_mut();
        }
    }

    // use im-rc if your contexts are too large to clone!
    // or, dangerously mutate and update subscriptions manually
    #[derive(Clone)]
    pub struct TodoManager {
        items: Vec<u32>,
    }

    impl AppContext<TodoManager> {
        fn remove_todo(&self, id: Uuid) {
            self.dispatch(|f| {})
        }

        async fn push_todo(&self, todo: TodoItem) {
            self.dispatch(|f| {
                //
                f.items.push(10);
            });
        }

        fn add_todo(&self) {
            // self.dispatch(|f| {});
            // let items = self.get(|f| &f.items);
        }
    }

    impl TodoManager {
        pub fn new() -> Self {
            todo!()
        }

        pub fn get_todo(&self) -> &TodoItem {
            todo!()
        }
    }

    pub struct TodoHandle {}
    impl TodoHandle {
        fn get_todo(&self, id: Uuid) -> &TodoItem {
            todo!()
        }

        fn add_todo(&self, todo: TodoItem) {}
    }

    // use_reducer, but exposes the reducer and context to children
    fn use_reducer_context() {}
    fn use_context_selector() {}

    fn use_context<'b, 'c, Root: 'static, Item: 'c>(
        ctx: &'b Context<'c>,
        f: impl Fn(Root) -> &'c Item,
    ) -> &'c Item {
        todo!()
    }

    pub fn use_todo_item<'b, 'c>(ctx: &'b Context<'c>, item: Uuid) -> &'c TodoItem {
        todo!()
        // ctx.use_hook(|| TodoManager::new(), |hook| {}, cleanup)
    }
    fn use_todos(ctx: &Context) -> TodoHandle {
        todo!()
    }

    fn use_todo_context(ctx: &Context) -> AppContext<TodoManager> {
        todo!()
    }

    fn test(ctx: Context) {
        let todos = use_todos(&ctx);
        let todo = todos.get_todo(Uuid::new_v4());

        let c = use_todo_context(&ctx);
        // todos.add_todo();
    }
}
