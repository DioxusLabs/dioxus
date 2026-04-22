//! Public enum accessors (is_variant / variant-downcast / transpose) should
//! remain reachable from downstream crates after the witness-seal plumbing was
//! extended to cover enums.

use dioxus_stores::*;
use dioxus_stores_visibility_helper::{PubEnum, PubEnumStoreExt};

#[allow(dead_code)]
fn uses_enum_accessors() {
    let store = use_store(PubEnum::new);
    let _: bool = store.is_a();
    let _: Option<Store<i32, _>> = store.a();
    let _ = store.transpose();
}

fn main() {}
