pub use std::cell::Cell;
use std::{cell::RefCell, thread::LocalKey};

#[derive(Debug)]
pub struct StoredItem {
    pub name: String,
    pub value: f32,
    pub items: Vec<String>,
}

thread_local! {
    pub static BAZ: RefCell<Option<StoredItem>> = const { RefCell::new(None) };
}

pub fn get_baz() -> &'static LocalKey<RefCell<Option<StoredItem>>> {
    if BAZ.with(|f| f.borrow().is_none()) {
        BAZ.set(Some(StoredItem {
            name: "BAR".to_string(),
            value: 0.0,
            items: vec!["item1".to_string(), "item2".to_string()],
        }));
    }

    &BAZ
}
