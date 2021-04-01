use crate::recoil::*;

pub static TODOS: AtomFamily<uuid::Uuid, TodoItem> = atom_family(|_| {});
pub static FILTER: Atom<FilterState> = atom(|_| FilterState::All);

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

impl crate::recoil::RecoilContext<()> {
    pub fn add_todo(&self, contents: String) {}
    pub fn remove_todo(&self) {}
    pub fn select_all_todos(&self) {}
    pub fn toggle_todo(&self, id: uuid::Uuid) {}
    pub fn clear_completed(&self) {}
    pub fn set_filter(&self, filter: &FilterState) {}
}
