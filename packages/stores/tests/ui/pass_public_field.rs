//! A downstream crate can call the `pub` field accessor.

use dioxus_stores::*;
use dioxus_stores_visibility_helper::{Item, ItemStoreExt};

#[allow(dead_code)]
fn uses_public_accessor() {
    let store = use_store(Item::new);
    let _: Store<i32, _> = store.public_field();
}

fn main() {}
