//! Typed view builders.
//!
//! Most applications use `rsx!`, but the generated HTML constructors can also
//! be used directly. A builder can collect attributes and children, then
//! [`ViewExt::into_vnode`] converts it into a VNode.

mod attribute;
mod child;
mod component;
mod element;
mod fragment;
mod text;
mod traits;
mod tuple;

pub use attribute::*;
pub use child::*;
pub use element::*;
pub use fragment::*;
pub use text::*;
pub use traits::*;

pub use crate::{static_attribute_value, static_text};
