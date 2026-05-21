//! The transposed struct must preserve private field visibility across crates.

use dioxus_stores::*;
use dioxus_stores_visibility_helper::{Item, ItemStoreExt};

fn main() {
    let store = use_store(Item::new);
    let _ = store.transpose().private_field;
}
