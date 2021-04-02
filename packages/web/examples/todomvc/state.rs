use crate::recoil::*;
use dioxus_core::prelude::Context;

pub static TODOS: AtomFamily<uuid::Uuid, TodoItem> = atom_family(|_| {});
pub static FILTER: Atom<FilterState> = atom(|_| FilterState::All);
pub static SHOW_ALL_TODOS: selector<bool> = selector(|g| g.getter(|f| false));

// an atomfamily is just a HashMap<K, Pin<Rc<V>>> that pins the Rc and exposes the values by reference
// we could do a more advanced management, but this is fine too

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

pub fn add_todo(ctx: &Context, contents: String) {}

pub fn remove_todo(ctx: &Context, id: &uuid::Uuid) {
    TODOS.with(&ctx).remove(id)
}

pub fn select_all_todos(ctx: &Context) {}

pub fn toggle_todo(ctx: &Context, id: &uuid::Uuid) {}

pub fn clear_completed(ctx: &Context) {
    let (set, get) = (self.set, self.get);

    TOODS
        .get(&ctx)
        .iter()
        .filter(|(k, v)| v.checked)
        .map(|(k, v)| TODOS.remove(&ctx, k));
}

pub fn set_filter(ctx: &Context, filter: &FilterState) {}

struct TodoManager<'a> {}
fn use_todos(ctx: &Context) {}

#[test]
fn test() {}
