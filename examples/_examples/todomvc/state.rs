use crate::recoil::*;

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

    pub fn remove_todo(&self, id: &uuid::Uuid) {
        // TODOS.with().remove(id)
    }

    pub fn select_all_todos(&self) {}

    pub fn toggle_todo(&self, id: &uuid::Uuid) {}

    pub fn clear_completed(&self) {
        // let (set, get) = (self.set, self.get);

        // TOODS
        //     .get(&cx)
        //     .iter()
        //     .filter(|(k, v)| v.checked)
        //     .map(|(k, v)| TODOS.remove(&cx, k));
    }

    pub fn set_filter(&self, filter: &FilterState) {}
}
