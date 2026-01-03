#![deny(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

pub mod android_plugin;
pub mod ios_plugin;
