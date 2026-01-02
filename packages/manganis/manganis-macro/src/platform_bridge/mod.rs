#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

mod android_plugin;
mod ios_plugin;
