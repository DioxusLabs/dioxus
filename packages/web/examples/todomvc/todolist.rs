use super::state::{FilterState, TodoItem, FILTER, TODOS};
use crate::filtertoggles::FilterToggles;
use crate::recoil::use_atom;
use crate::todoitem::TodoEntry;
use dioxus_core::prelude::*;

pub fn TodoList(ctx: Context, props: &()) -> DomTree {
    let (entry, set_entry) = use_state(&ctx, || "".to_string());
    let todos: &Vec<TodoItem> = todo!();
    let filter = use_atom(&ctx, &FILTER);

    let list = todos
        .iter()
        .filter(|f| match filter {
            FilterState::All => true,
            FilterState::Active => !f.checked,
            FilterState::Completed => f.checked,
        })
        .map(|item| {
            rsx!(TodoEntry {
                key: "{order}",
                id: item.id,
            })
        });

    ctx.render(rsx! {
        div {
            // header
            header {
                class: "header"
                h1 {"todos"}
                input {
                    class: "new-todo"
                    placeholder: "What needs to be done?"
                    value: "{entry}"
                    oninput: move |evt| set_entry(evt.value)
                }
            }

            // list
            {list}

            // filter toggle (show only if the list isn't empty)
            {(!todos.is_empty()).then(||
                rsx!{ FilterToggles {}
            })}
        }
    })
}
