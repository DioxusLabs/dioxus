use super::state::TODOS;
use crate::recoil::use_atom_family;
use dioxus_core::prelude::*;

#[derive(PartialEq, Props)]
pub struct TodoEntryProps {
    id: uuid::Uuid,
}

pub fn TodoEntry(cx: Context, props: &TodoEntryProps) -> VNode {
    let (is_editing, set_is_editing) = use_state(cx, || false);
    let todo = use_atom_family(&cx, &TODOS, cx.id);

    cx.render(rsx! (
        li {
            "{todo.id}"
            input {
                class: "toggle"
                type: "checkbox"
                "{todo.checked}"
            }
            {is_editing.then(|| rsx!(
                input {
                    value: "{todo.contents}"
                }
            ))}
        }
    ))
}
