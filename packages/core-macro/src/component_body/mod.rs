//! This module is used for parsing a component function into a struct that is subsequently
//! deserialized into something useful using deserializer arguments.
//!
//! Let's break that down with a term glossary and examples which show usage and implementing.
//!
//! # Glossary
//! * `component body` - The [`ComponentBody`] struct. It's used to parse a component function [`proc_macro::TokenStream`]
//! to a reusable struct that deserializers use to modify the token stream.
//! * `deserializer` - A struct that deserializes the [`ComponentBody`] into a [`DeserializerOutput`].
//! It implements the [`DeserializerArgs`] trait, but as you can see, it's called "DeserializerArgs",
//! not "Deserializer". Why?
//! Because "args" makes more sense to the caller of [`ComponentBody::deserialize`], which
//! takes an [`DeserializerArgs`] argument. However, you can think of "DeserializerArgs" as the deserializer.
//! * `deserializer output` - A struct that implements the [`DeserializerOutput`] trait.
//! This struct is what enables deserializers to use each other, since it contains the fields that
//! a deserializer needs to turn a token stream to a different token stream.
//! This means a deserializer can get the output of another deserializer, and use that output,
//! thereby using the functionality of a different deserializer.
//! This struct also implements [`ToTokens`], which means that this is the final stage of the whole process.
//!
//! # Examples
//! *Not all imports might be included.*
//!
//! ## Usage in a procedural macro attribute
//! ```rs,ignore
//! use proc_macro::TokenStream;
//!
//! // Some documentation. You can reuse this in your deserializer structs.
//! /// This attribute changes the name of a component function to whatever the first argument is.
//! #[proc_macro_attribute]
//! pub fn name_changer(args: TokenStream, input: TokenStream) -> TokenStream {
//!     // Parse the component body.
//!     let component_body = parse_macro_input!(input as ComponentBody);
//!
//!     // Parse the first argument, which is going to be the components new name.
//!     let new_name: String = match Punctuated::<Path, Token![,]>::parse_terminated.parse(args) {
//!         Err(e) => return e.to_compile_error().into(), // Convert to a compile error and return
//!         Ok(args) => {
//!             // If the argument exists, then convert it to a string
//!             if let Some(first) = args.first() {
//!                 first.to_token_stream().to_string()
//!             } else {
//!                 // If the argument doesn't exist, return an error with the appropriate message.
//!                 // The "span" is the location of some code.
//!                 // The error occurred in the "args" token stream, so point the error there.
//!                 return Error::new(args.span(), "No new name provided").to_compile_error().into();
//!             }
//!         }
//!     };
//!
//!     let new_name = &*new_name;
//!
//!     // Deserialize the component body to an output with the given args.
//!     let output = component_body.deserialize(NameChangerDeserializerArgs { new_name });
//!
//!     // Error handling like before, except now you're ready to return the final value.
//!     match output {
//!         Err(e) => e.to_compile_error().into(),
//!         Ok(output) => output.to_token_stream().into(),
//!     }
//! }
//! ```
//! ## Using the macro in Dioxus code:
//! ```rs
//! use your_proc_macro_library::name_changer;
//! use dioxus::prelude::*;
//!
//! #[name_changer(CoolName)]
//! pub fn LameName(cx: Scope) -> Element {
//!     render! { "I want a cool name!" }
//! }
//!
//! pub fn App(cx: Scope) -> Element {
//!     render! { CoolName {} } // Renders: "I want a cool name!"
//! }
//! ```
//! ## Implementing a component body deserializer
//! ```rs
//! use syn::{Result, ItemFn, Signature, Ident};
//! use quote::quote;
//!
//! // Create a list of arguments.
//! // If there was no args, just make it empty. The "args" struct is also the deserializer struct.
//! // For the docs, you can basically copy paste this text and replace "name_changer" with your macro path.
//! // Although unfortunately, the link does not work
//! // Just make sure that your macro is well documented.
//! /// The args and deserializing implementation for the [`name_changer`] macro.
//! #[derive(Clone)]
//! pub struct NameChangerDeserializerArgs<'a> {
//!     pub new_name: &'a str,
//! }
//!
//! // Create an output struct.
//! // The ItemFn represents a modified component function.
//! // To read what fields should be here, check out the `DeserializerOutput` struct docs.
//! // For the docs, you can basically copy paste this text and replace "name_changer" with your macro path.
//! // Just make sure that your macro is well documented.
//! /// The output fields and [`ToTokens`] implementation for the [`name_changer`] macro.
//! #[derive(Clone)]
//! pub struct NameChangerDeserializerOutput {
//!     pub comp_fn: ItemFn,
//! }
//!
//! // Implement `ToTokens`, which is forced by `DeserializerOutput`.
//! // This will usually be very simple like this, even for complex deserializers.
//! // That's because of the way the `DeserializerOutput` is designed.
//! impl ToTokens for NameChangerDeserializerOutput {
//!     fn to_tokens(&self, tokens: &mut TokenStream) {
//!         let comp_fn = &self.comp_fn;
//!
//!         tokens.append_all(quote! {
//!             #comp_fn
//!         });
//!     }
//! }
//!
//! impl DeserializerOutput for NameChangerDeserializerOutput {}
//!
//! // Implement `DeserializerArgs`. This is the core part of deserializers.
//! impl<'a> DeserializerArgs<NameChangerDeserializerOutput> for NameChangerDeserializerArgs<'a> {
//!     fn to_output(&self, component_body: &ComponentBody) -> Result<NameChangerDeserializerOutput> {
//!         let old_fn = &component_body.item_fn;
//!         let old_sig = &old_fn.sig;
//!
//!         // For more complex uses, you will probably use `quote::parse_quote!` in combination with
//!         // creating the structs manually.
//!         // However, create the structs manually if you can.
//!         // It's more reliable, because you only modify a certain struct field
//!         // and set the others to be the clone of the original component body.
//!         // That ensures that no information will be accidentally removed.
//!         let new_sig = Signature {
//!             ident: Ident::new(self.new_name, old_sig.ident.span()),
//!             ..old_sig.clone()
//!         };
//!         let new_fn = ItemFn {
//!             sig: new_sig,
//!             ..old_fn.clone()
//!         };
//!
//!         Ok(NameChangerDeserializerOutput {
//!             comp_fn: new_fn
//!         })
//!     }
//! ```

use dioxus_core::{Element, Scope};
use quote::ToTokens;
use syn::{parse_quote, Path, Type};

mod component_body;
pub use crate::component_body::component_body::ComponentBody;

/// The output produced by a deserializer.
///
/// # For implementors
/// Struct field guidelines:
/// * Must be public, so that other deserializers can utilize them.
/// * Should usually be [`Item`]s that you then simply combine in a [`quote!`]
/// in the [`ComponentBodyDeserializer::output_to_token_stream2`] function.
/// * If an [`Item`] might not be included, wrap it in an [`Option`].
/// * Must be at the component function "level"/"context".
/// For example, the [`InlinePropsDeserializer`](crate::component_body_deserializers::inline_props::InlinePropsDeserializer)
/// produces two [`Item`]s; the function but with arguments turned into props, and the props struct.
/// It does not return any [`Item`]s inside the struct or function.
pub trait DeserializerOutput: ToTokens {}

/// The args passed to a [`ComponentBody`] when deserializing it.
///
/// It's also the struct that does the deserializing.
/// It's called "DeserializerArgs", not "Deserializer". Why?
/// Because "args" makes more sense to the caller of [`ComponentBody::deserialize`], which
/// takes an [`DeserializerArgs`] argument. However, you can think of "DeserializerArgs" as the deserializer.
pub trait DeserializerArgs<TOutput>: Clone
    where
        TOutput: DeserializerOutput,
{
    // There's a lot of Results out there... let's make sure that this is a syn::Result.
    // Let's also make sure there's not a warning.
    /// Creates a [`DeserializerOutput`] from the `self` args and a [`ComponentBody`].
    /// The [`ComponentBody::deserialize`] provides a cleaner way of calling this function.
    /// As a result, don't make this public when you implement it.
    #[allow(unused_qualifications)]
    fn to_output(&self, component_body: &ComponentBody) -> syn::Result<TOutput>;
}

pub(crate) trait TypeHelper {
    fn get_path() -> Path;
    fn get_path_string() -> String {
        Self::get_path().to_token_stream().to_string()
    }
}

impl<'a> TypeHelper for Scope<'a> {
    fn get_path() -> Path {
        parse_quote!(::dioxus::core::Scope)
    }
}

impl<'a> TypeHelper for Element<'a> {
    fn get_path() -> Path {
        parse_quote!(::dioxus::core::Element)
    }
}

/// Checks if a given generic that implements [`TypeHelper`] is equal to a [`Type`].
///
/// Returns `true` for both an imported and fully qualified [`TypeHelper::get_path`],
/// but not "partially" qualified, such as: `use dioxus::core; core::Scope`.
///
/// # Edge cases
/// These are caused by macro limitations.
/// It's impossible to tell what module a type is imported from and if that type is an alias.
/// Examples of some edge cases:
/// * `is_type_eq<dioxus_core::Scope>(t) == true` where `t == type Scope = u8;` (an alias).
/// * `is_type_eq<dioxus_core::Scope>(t) == false` where `t == core::Scope` from `use dioxus::core;`
/// - It's possible to return `true` here, but `core` is too ambiguous,
/// and as mentioned, it's impossible to know if a `core` belongs to `dioxus` or something else.
fn is_type_eq<T>(input: &Type) -> bool
where
    T: TypeHelper,
{
    let scope_path = T::get_path();
    let input_path: Path = parse_quote!(#input);
    let scope_path_segs = scope_path.segments;
    let input_path_segs = input_path.segments;
    let input_seg_len = input_path_segs.len();

    return if input_seg_len > 1 {
        if input_seg_len == scope_path_segs.len() {
            for i in 0..input_seg_len {
                let input_seg = &input_path_segs[i];
                let scope_seg = &scope_path_segs[i];

                if input_seg.ident != scope_seg.ident {
                    return false;
                }
            }

            true
        } else {
            false
        }
    } else if input_seg_len == 1 {
        let scope_last_seg = scope_path_segs.last().unwrap();
        let input_seg = input_path_segs.first().unwrap();

        scope_last_seg.ident == input_seg.ident
    } else {
        false
    };
}