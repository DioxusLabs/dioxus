fn main() {}

pub mod dioxus {
    pub mod prelude {
        pub unsafe trait Properties {
            type Builder;
            const CAN_BE_MEMOIZED: bool;
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
