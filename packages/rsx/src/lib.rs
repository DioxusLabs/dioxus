#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

//! Parse the root tokens in the rsx! { } macro
//! =========================================
//!
//! This parsing path emerges directly from the macro call, with `RsxRender` being the primary entrance into parsing.
//! This feature must support:
//! - [x] Optionally rendering if the `in XYZ` pattern is present
//! - [x] Fragments as top-level element (through ambiguous)
//! - [x] Components as top-level element (through ambiguous)
//! - [x] Tags as top-level elements (through ambiguous)
//! - [x] Good errors if parsing fails
//!
//! Any errors in using rsx! will likely occur when people start using it, so the first errors must be really helpful.
//!
mod attribute;
mod component;
mod element;
mod forloop;
mod ifchain;
mod node;
mod raw_expr;
mod rsx_block;
mod rsx_call;
mod template_body;
mod text_node;

mod diagnostics;
mod expr_node;
mod ifmt;
mod literal;
mod location;
mod partial_closure;
mod util;

// Public API used by the proc macro and workspace tooling. Keep parser-internal
// helpers in `innerlude` instead of glob-reexporting the whole crate.
pub use attribute::{Attribute, AttributeName, AttributeValue, IfAttributeValue, Spread};
pub use component::Component;
pub use diagnostics::Diagnostics;
pub use element::{Element, ElementName};
pub use expr_node::ExprNode;
pub use forloop::ForLoop;
pub use ifchain::IfChain;
pub use ifmt::{FormattedSegment, IfmtInput, Segment};
pub use literal::{HotLiteral, HotReloadFormattedSegment};
pub use location::DynIdx;
pub use node::BodyNode;
pub use partial_closure::PartialClosure;
pub use raw_expr::PartialExpr;
pub use rsx_call::CallBody;
pub use template_body::TemplateBody;
pub use text_node::TextNode;

use quote::{ToTokens, TokenStreamExt, quote};
use syn::{
    Result, Token,
    parse::{Parse, ParseStream},
};

pub use innerlude::*;
pub(crate) mod innerlude {
    pub use crate::attribute::*;
    pub use crate::component::*;
    pub use crate::element::*;
    pub use crate::expr_node::*;
    pub use crate::forloop::*;
    pub use crate::ifchain::*;
    pub use crate::node::*;
    pub use crate::raw_expr::*;
    pub use crate::rsx_block::*;
    pub use crate::template_body::*;
    pub use crate::text_node::*;

    pub use crate::diagnostics::*;
    pub use crate::ifmt::*;
    pub use crate::literal::*;
    pub use crate::util::*;
}
