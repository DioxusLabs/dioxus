use super::state::{FilterState, TodoItem, FILTER, TODOS};
use crate::filtertoggles;
use crate::recoil::use_atom;
use crate::todoitem::TodoEntry;
use dioxus_core::prelude::*;

pub fn TodoList(ctx: Context, props: &()) -> DomTree {
    let (draft, set_draft) = use_state(&ctx, || "".to_string());
    let (todos, _) = use_state(&ctx, || Vec::<TodoItem>::new());
    let filter = use_atom(&ctx, &FILTER);

    let list = todos
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
        });

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

            {list}

            // filter toggle (show only if the list isn't empty)
            {(!todos.is_empty()).then(||
                rsx!( filtertoggles::FilterToggles {})
            )}
        }
    })
}
