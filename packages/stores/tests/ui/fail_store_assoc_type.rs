use dioxus_stores::*;

#[derive(Store)]
struct Item {
    value: i32,
}

#[store]
impl Store<Item> {
    type Hidden = i32;
}

fn main() {
    let _ = Item { value: 0 };
}
