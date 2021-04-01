use dioxus_core::prelude::*;
use dioxus_web::WebsysRenderer;
use recoil::{use_recoil_callback, RecoilContext};
use uuid::Uuid;

static TODOS: AtomFamily<Uuid, TodoItem> = atom_family(|_| {});

fn main() {
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(|ctx, props| {
        let global_reducer = use_recoil_callback(|| ());

        let todos = use_atom(TODOS).iter().map(|(order, item)| {
            rsx!(TodoItem {
                key: "{order}",
                id: item.id,
            })
        });

        ctx.render(rsx! {
            div {
                {todos}
                Footer {}
            }
        })
    }))
}

#[derive(Debug, PartialEq, Clone)]
pub struct TodoItem {
    pub id: Uuid,
    pub checked: bool,
    pub contents: String,
}

// build a global context for the app
// as we scale the app, we can create separate, stateful impls
impl RecoilContext<()> {
    fn add_todo(&self) {}
    fn remove_todo(&self) {}
    fn select_all_todos(&self) {}
}

mod hooks {
    use super::*;
    fn use_keyboard_shortcuts(ctx: &Context) {}
}

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

    fn atom() {}

    // struct Dispatcher {}

    struct AppContext<T: Clone> {
        _t: std::rc::Rc<T>,
    }

    // pub fn use_appcontext<T: Clone>(ctx: &Context, f: impl FnOnce() -> T) -> AppContext<T> {
    //     todo!()
    // }

    // static TodoList: ContextFamily = context_family();

    // struct TodoBoss<'a> {}

    // fn use_recoil_todos() -> TodoBoss {}

    // pub fn use_context_family(ctx: &Context) {}

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

    // // use im-rc if your contexts are too large to clone!
    // // or, dangerously mutate and update subscriptions manually
    // #[derive(Clone, Debug, PartialEq)]
    // pub struct TodoManager {
    //     items: Vec<u32>,
    // }

    // // App context is an ergonomic way of sharing data models through a tall tree
    // // Because it holds onto the source data with Rc, it's cheap to clone through props and allows advanced memoization
    // // It's particularly useful when moving through tall trees, or iterating through complex data models.
    // // By wrapping the source type, we can forward any mutation through "dispatch", making it clear when clones occur.
    // // This also enables traditional method-style
    // impl AppContext<TodoManager> {
    //     fn get_todos(&self, ctx: &Context) {}

    //     fn remove_todo(&self, id: Uuid) {
    //         self.dispatch(|f| {
    //             // todos... remove
    //         })
    //     }

    //     async fn push_todo(&self, todo: TodoItem) {
    //         self.dispatch(|f| {
    //             //
    //             f.items.push(10);
    //         });
    //     }

    //     fn add_todo(&self) {
    //         // self.dispatch(|f| {});
    //         // let items = self.get(|f| &f.items);
    //     }
    // }

    // pub enum TodoActions {}
    // impl TodoManager {
    //     pub fn reduce(s: &mut Rc<Self>, action: TodoActions) {
    //         match action {
    //             _ => {}
    //         }
    //     }

    //     pub fn new() -> Rc<Self> {
    //         todo!()
    //     }

    //     pub fn get_todo(&self, id: Uuid) -> &TodoItem {
    //         todo!()
    //     }

    //     pub fn get_todos(&self) -> &BTreeMap<String, TodoItem> {
    //         todo!()
    //     }
    // }

    // pub struct TodoHandle {}
    // impl TodoHandle {
    //     fn get_todo(&self, id: Uuid) -> &TodoItem {
    //         todo!()
    //     }

    //     fn add_todo(&self, todo: TodoItem) {}
    // }

    // // use_reducer, but exposes the reducer and context to children
    // fn use_reducer_context() {}
    // fn use_context_selector() {}

    // fn use_context<'b, 'c, Root: 'static, Item: 'c>(
    //     ctx: &'b Context<'c>,
    //     f: impl Fn(Root) -> &'c Item,
    // ) -> &'c Item {
    //     todo!()
    // }

    // pub fn use_todo_item<'b, 'c>(ctx: &'b Context<'c>, item: Uuid) -> &'c TodoItem {
    //     todo!()
    //     // ctx.use_hook(|| TodoManager::new(), |hook| {}, cleanup)
    // }
    // fn use_todos(ctx: &Context) -> TodoHandle {
    //     todo!()
    // }

    // fn use_todo_context(ctx: &Context) -> AppContext<TodoManager> {
    //     todo!()
    // }

    // fn test(ctx: Context) {
    //     let todos = use_todos(&ctx);
    //     let todo = todos.get_todo(Uuid::new_v4());

    //     let c = use_todo_context(&ctx);
    //     // todos.add_todo();
    // }
}

mod recoil {

    pub struct RecoilContext<T: 'static> {
        _inner: T,
    }

    impl<T: 'static> RecoilContext<T> {
        /// Get the value of an atom. Returns a reference to the underlying data.

        pub fn get(&self) {}

        /// Replace an existing value with a new value
        ///
        /// This does not replace the value instantly, and all calls to "get" within the current scope will return
        pub fn set(&self) {}

        // Modify lets you modify the value in place. However, because there's no previous value around to compare
        // the new one with, we are unable to memoize the change. As such, all downsteam users of this Atom will
        // be updated, causing all subsrcibed components to re-render.
        //
        // This is fine for most values, but might not be performant when dealing with collections. For collections,
        // use the "Family" variants as these will stay memoized for inserts, removals, and modifications.
        //
        // Note - like "set" this won't propogate instantly. Once all "gets" are dropped, only then will we run the
        pub fn modify(&self) {}
    }

    pub fn use_recoil_callback<G>(f: impl Fn() -> G) -> RecoilContext<G> {
        todo!()
    }
}
