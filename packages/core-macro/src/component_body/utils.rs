use crate::component_body::ComponentBody;
use dioxus_core::{Element, Scope};
use quote::ToTokens;
use std::cmp::Ordering;
use syn::{parse_quote, Path, Type};

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

pub trait TypeHelper {
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
pub fn is_type_eq<T>(input: &Type) -> bool
where
    T: TypeHelper,
{
    let scope_path = T::get_path();
    let input_path: Path = parse_quote!(#input);
    let scope_path_segs = scope_path.segments;
    let input_path_segs = input_path.segments;
    let input_seg_len = input_path_segs.len();

    match input_seg_len.cmp(&1) {
        Ordering::Less => false,
        Ordering::Equal => {
            let scope_last_seg = scope_path_segs.last().unwrap();
            let input_seg = input_path_segs.first().unwrap();

            scope_last_seg.ident == input_seg.ident
        }
        Ordering::Greater => {
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
        }
    }
}
