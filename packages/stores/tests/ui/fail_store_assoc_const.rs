use dioxus_stores::*;

#[derive(Store)]
struct Item {
    value: i32,
}

#[store]
impl Store<Item> {
    const SECRET: i32 = 7;
}

fn main() {
    let _ = Item { value: 0 };
}
