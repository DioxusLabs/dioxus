#![allow(non_snake_case)]

//! # Dioxus Namespace for HTML
//!
//! This crate provides a set of compile-time correct HTML elements that can be used with the Rsx and Html macros.
//! This system allows users to easily build new tags, new types, and customize the output of the Rsx and Html macros.
//!
//! An added benefit of this approach is the ability to lend comprehensive documentation on how to use these elements inside
//! of the Rsx and Html macros. Each element comes with a substantial amount of documentation on how to best use it, hopefully
//! making the development cycle quick.
//!
//! All elements are used as zero-sized unit structs with trait impls.
//!
//! Currently, we don't validate for structures, but do validate attributes.

mod elements;
mod events;
pub mod geometry;
mod global_attributes;
pub mod input_data;
#[cfg(feature = "wasm-bind")]
mod web_sys_bind;

pub use elements::*;
pub use events::*;
pub use global_attributes::*;

#[macro_export]
macro_rules! custom_elements {
    (
        $( $ele:ident( $tag:expr, $( $attr:ident ),* ); )+
    ) => {$(
        #[allow(non_camel_case_types)]
        pub struct $ele;

        impl DioxusElement for $ele {
            const TAG_NAME: &'static str = $tag;
            const NAME_SPACE: Option<&'static str> = None;
        }
        impl GlobalAttributes for $ele {}
        impl $ele {$(
            #[allow(non_upper_case_globals)]
            pub const $attr: AttributeDiscription = AttributeDiscription {
                name: stringify!($attr),
                namespace: None,
                volatile: false,
            };
        )*}
    )+}
}
