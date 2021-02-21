//! Demonstrate that borrowed data is possible as a property type
//! Borrowing (rather than cloning) is very important for speed and ergonomics.

fn main() {}

use dioxus_core::prelude::*;

struct Props {
    items: Vec<ListItem>,
}

struct ListItem {
    name: String,
    age: u32,
}

fn app(ctx: Context, props: &Props) -> DomTree {
    ctx.view(move |b| {
        let mut root = builder::div(b);
        for child in &props.items {
            // notice that the child directly borrows from our vec
            // this makes lists very fast (simply views reusing lifetimes)
            root = root.child(builder::virtual_child(
                b,
                ChildProps { item: child },
                child_item,
            ));
        }
        root.finish()
    })
}

struct ChildProps<'a> {
    item: &'a ListItem,
}

fn child_item(ctx: Context, props: &ChildProps) -> DomTree {
    todo!()
}
