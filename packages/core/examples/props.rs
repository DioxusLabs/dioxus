use dioxus_core::prelude::*;

#[derive(Debug, PartialEq, Props)]
struct SomeProps {
    a: i32,

    // automatically do the default (none) and automatically Into<T>
    #[builder(default, setter(strip_option))]
    b: Option<i32>,
}

fn main() {
    let g: SomeProps = SomeProps::builder().a(10).b(10).build();

    let _r = g.b.unwrap_or_else(|| 10);
}
