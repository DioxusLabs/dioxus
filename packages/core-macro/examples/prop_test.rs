fn main() {}

pub mod dioxus {
    pub mod prelude {
        pub trait Properties {
            type Builder;
            const IS_STATIC: bool;
            fn builder() -> Self::Builder;
            unsafe fn memoize(&self, other: &Self) -> bool;
        }
    }
}

/// This implementation should require a "PartialEq" because it memoizes (no external references)
#[derive(PartialEq, dioxus_core_macro::Props)]
struct SomeProps {
    a: String,
}

/// This implementation does not require a "PartialEq" because it does not memoize
#[derive(dioxus_core_macro::Props)]
struct SomePropsTwo<'a> {
    a: &'a str,
}
