fn main() {}

pub mod dioxus {
    pub mod prelude {
        pub trait Properties {
            type Builder;
            fn builder() -> Self::Builder;
        }
    }
}
#[derive(dioxus_core_macro::Props)]
struct SomeProps {
    a: String,
}

#[derive(dioxus_core_macro::Props)]
struct SomePropsTwo<'a> {
    a: &'a str,
}
