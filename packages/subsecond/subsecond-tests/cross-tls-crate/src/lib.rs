pub use std::cell::Cell;
use std::{cell::RefCell, thread::LocalKey};

#[derive(Debug)]
pub struct StoredItem {
    pub name: String,
    pub value: f32,
    pub items: Vec<String>,
}

thread_local! {
    pub static BAR: RefCell<Option<StoredItem>> = const { RefCell::new(None) };
}

pub fn get_bar() -> &'static LocalKey<RefCell<Option<StoredItem>>> {
    if BAR.with(|f| f.borrow().is_none()) {
        BAR.set(Some(StoredItem {
            name: "BAR".to_string(),
            value: 0.0,
            items: vec!["item1".to_string(), "item2".to_string()],
        }));
    }

    &BAR
}
