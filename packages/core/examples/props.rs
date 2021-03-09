use dioxus_core_macro::Props;

#[derive(Debug, Props)]
struct SomeProps {
    a: i32,

    // automatically do the default (none) and automatically Into<T>
    #[builder(default, setter(strip_option))]
    b: Option<i32>,
}

// have we committed to the trait style yet?

fn main() {
    let g: SomeProps = SomeProps::builder().a(10).b(10).build();

    let r = g.b.unwrap_or_else(|| 10);
}

fn auto_into_some() {}
