//! pretty printer for rsx code

use dioxus_rsx::*;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;

mod block;
mod buffer;
mod children;
mod component;
mod element;
mod expr;
mod ident;
mod util;

pub use block::{fmt_block, get_format_blocks};
pub use ident::write_ident;
