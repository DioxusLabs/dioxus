use dioxus_core::prelude::*;
use dioxus_web::WebsysRenderer;

static APP_STYLE: &'static str = include_str!("./todomvc/style.css");

fn main() {
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(App));
}
// =======================
// state-related items
// =======================
pub static TODOS: AtomFamily<uuid::Uuid, TodoItem> = atom_family(|_| {});
pub static FILTER: Atom<FilterState> = atom(|_| FilterState::All);
pub static SHOW_ALL_TODOS: selector<bool> = selector(|g| g.getter(|f| false));

#[derive(PartialEq)]
pub enum FilterState {
    All,
    Active,
    Completed,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TodoItem {
    pub id: uuid::Uuid,
    pub checked: bool,
    pub contents: String,
}

impl RecoilContext<()> {
    pub fn add_todo(&self, contents: String) {}
    pub fn remove_todo(&self, id: &uuid::Uuid) {}
    pub fn select_all_todos(&self) {}
    pub fn toggle_todo(&self, id: &uuid::Uuid) {}
    pub fn clear_completed(&self) {}
    pub fn set_filter(&self, filter: &FilterState) {}
}

// =======================
// Components
// =======================
pub fn App(ctx: Context, props: &()) -> DomTree {
    ctx.render(rsx! {
        div {
            id: "app"
            style { "{APP_STYLE}" }

            // list
            TodoList {}

            // footer
            footer {
                class: "info"
                p {"Double-click to edit a todo"}
                p {
                    "Created by "
                    a { "jkelleyrtp", href: "http://github.com/jkelleyrtp/" }
                }
                p {
                    "Part of "
                    a { "TodoMVC", href: "http://todomvc.com" }
                }
            }
        }
    })
}

pub fn TodoList(ctx: Context, props: &()) -> DomTree {
    let (draft, set_draft) = use_state(&ctx, || "".to_string());
    let (todos, _) = use_state(&ctx, || Vec::<TodoItem>::new());
    let filter = use_atom(&ctx, &FILTER);

    ctx.render(rsx! {
        div {
            header {
                class: "header"
                h1 {"todos"}
                input {
                    class: "new-todo"
                    placeholder: "What needs to be done?"
                    value: "{draft}"
                    oninput: move |evt| set_draft(evt.value)
                }
            }

            { // list
                todos
                .iter()
                .filter(|item| match filter {
                    FilterState::All => true,
                    FilterState::Active => !item.checked,
                    FilterState::Completed => item.checked,
                })
                .map(|item| {
                    rsx!(TodoEntry {
                        key: "{order}",
                        id: item.id,
                    })
                })
            }

            // filter toggle (show only if the list isn't empty)
            {(!todos.is_empty()).then(||
                rsx!( FilterToggles {})
            )}
        }
    })
}

#[derive(PartialEq, Props)]
pub struct TodoEntryProps {
    id: uuid::Uuid,
}

pub fn TodoEntry(ctx: Context, props: &TodoEntryProps) -> DomTree {
    let (is_editing, set_is_editing) = use_state(&ctx, || false);
    let todo = use_atom_family(&ctx, &TODOS, props.id);

    ctx.render(rsx! (
        li {
            "{todo.id}"
            input {
                class: "toggle"
                type: "checkbox"
                "{todo.checked}"
            }
            {is_editing.then(|| rsx!(
                input {
                    value: "{contents}"
                }
            ))}
        }
    ))
}

pub fn FilterToggles(ctx: Context, props: &()) -> DomTree {
    let reducer = recoil::use_callback(&ctx, || ());
    let items_left = recoil::use_atom_family(&ctx, &TODOS, uuid::Uuid::new_v4());

    let toggles = [
        ("All", "", FilterState::All),
        ("Active", "active", FilterState::Active),
        ("Completed", "completed", FilterState::Completed),
    ]
    .iter()
    .map(|(name, path, filter)| {
        rsx!(
            li {
                class: "{name}"
                a {
                    href: "{path}"
                    onclick: move |_| reducer.set_filter(&filter)
                    "{name}"
                }
            }
        )
    });

    // todo
    let item_text = "";
    let items_left = "";

    ctx.render(rsx! {
        footer {
            span {
                strong {"{items_left}"}
                span {"{item_text} left"}
            }
            ul {
                class: "filters"
                {toggles}
            }
        }
    })
}

pub use recoil::*;
mod recoil {
    use dioxus_core::context::Context;

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

    pub fn use_callback<'a, G>(c: &Context<'a>, f: impl Fn() -> G) -> &'a RecoilContext<G> {
        todo!()
    }

    pub fn use_atom<T: PartialEq, O>(c: &Context, t: &'static Atom<T>) -> O {
        todo!()
    }
    pub fn use_batom<T: PartialEq, O>(c: &Context, t: impl Readable) -> O {
        todo!()
    }

    pub trait Readable {}
    impl<T: PartialEq> Readable for &'static Atom<T> {}
    impl<K: PartialEq, V: PartialEq> Readable for &'static AtomFamily<K, V> {}

    pub fn use_atom_family<'a, K: PartialEq, V: PartialEq>(
        c: &Context<'a>,
        t: &'static AtomFamily<K, V>,
        g: K,
    ) -> &'a V {
        todo!()
    }

    pub use atoms::{atom, Atom};
    pub use atoms::{atom_family, AtomFamily};
    mod atoms {

        use super::*;
        pub struct AtomBuilder<T: PartialEq> {
            pub key: String,
            pub manual_init: Option<Box<dyn Fn() -> T>>,
            _never: std::marker::PhantomData<T>,
        }

        impl<T: PartialEq> AtomBuilder<T> {
            pub fn new() -> Self {
                Self {
                    key: uuid::Uuid::new_v4().to_string(),
                    manual_init: None,
                    _never: std::marker::PhantomData {},
                }
            }

            pub fn init<A: Fn() -> T + 'static>(&mut self, f: A) {
                self.manual_init = Some(Box::new(f));
            }

            pub fn set_key(&mut self, _key: &'static str) {}
        }

        pub struct atom<T: PartialEq>(pub fn(&mut AtomBuilder<T>) -> T);
        pub type Atom<T: PartialEq> = atom<T>;

        pub struct AtomFamilyBuilder<K, V> {
            _never: std::marker::PhantomData<(K, V)>,
        }

        pub struct atom_family<K: PartialEq, V: PartialEq>(pub fn(&mut AtomFamilyBuilder<K, V>));
        pub type AtomFamily<K: PartialEq, V: PartialEq> = atom_family<K, V>;
    }

    pub use selectors::selector;
    mod selectors {
        pub struct SelectorBuilder<Out, const Built: bool> {
            _p: std::marker::PhantomData<Out>,
        }
        impl<O> SelectorBuilder<O, false> {
            pub fn getter(self, f: impl Fn(()) -> O) -> SelectorBuilder<O, true> {
                todo!()
                // std::rc::Rc::pin(value)
                // todo!()
            }
        }
        pub struct selector<O>(pub fn(SelectorBuilder<O, false>) -> SelectorBuilder<O, true>);
    }
}
