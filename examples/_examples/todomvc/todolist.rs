use crate::{
    filtertoggles,
    recoil::use_atom,
    state::{FilterState, TodoItem, FILTER, TODOS},
    todoitem::TodoEntry,
};
use dioxus_core::prelude::*;

pub fn TodoList(cx: Context<()>) -> VNode {
    let (draft, set_draft) = use_state(cx, || "".to_string());
    let (todos, _) = use_state(cx, || Vec::<TodoItem>::new());
    let filter = use_atom(&cx, &FILTER);

    cx.render(rsx! {
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
                rsx!( filtertoggles::FilterToggles {})
            )}
        }
    })
}
