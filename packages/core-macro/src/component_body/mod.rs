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
//! pub fn LameName() -> Element {
//!     rsx! { "I want a cool name!" }
//! }
//!
//! pub fn App() -> Element {
//!     rsx! { CoolName {} } // Renders: "I want a cool name!"
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

pub mod utils;

pub use utils::DeserializerArgs;
pub use utils::DeserializerOutput;

use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::*;

/// General struct for parsing a component body.
/// However, because it's ambiguous, it does not implement [`ToTokens`](quote::to_tokens::ToTokens).
///
/// Refer to the [module documentation](crate::component_body) for more.
pub struct ComponentBody {
    /// The component function definition. You can parse this back into a [`ComponentBody`].
    /// For example, you might modify it, parse it into a [`ComponentBody`], and deserialize that
    /// using some deserializer. This is how deserializers use other deserializers.
    ///
    /// **`item_fn.sig.inputs` includes the context argument!**
    /// Keep this in mind when creating deserializers, because you often might want to ignore it.
    /// That might be annoying, but it would be bad design for this kind of struct to not be parsable from itself.
    pub item_fn: ItemFn,
    /// If the function has any arguments other than the context.
    pub has_extra_args: bool,
}

impl ComponentBody {
    /// Deserializes the body into the [`TOutput`] with the specific [`TArgs`].
    /// Even if the args are empty, the [`TArg`] type still determines what [`TOutput`] will be generated.
    pub fn deserialize<TOutput, TArgs>(&self, args: TArgs) -> Result<TOutput>
    where
        TOutput: DeserializerOutput,
        TArgs: DeserializerArgs<TOutput>,
    {
        args.to_output(self)
    }
}

impl Parse for ComponentBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let item_fn: ItemFn = input.parse()?;

        let element_type_path = "dioxus_core::Element";

        if item_fn.sig.output == ReturnType::Default {
            return Err(Error::new(
                item_fn.sig.output.span(),
                format!("Must return a <{}>", element_type_path),
            ));
        }

        let has_extra_args = !item_fn.sig.inputs.is_empty();

        Ok(Self {
            item_fn,
            has_extra_args,
        })
    }
}
