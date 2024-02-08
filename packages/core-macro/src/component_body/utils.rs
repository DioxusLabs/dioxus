use crate::component_body::ComponentBody;
use quote::ToTokens;

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

impl<T: ToTokens> DeserializerOutput for T {}

/// The args passed to a [`ComponentBody`] when deserializing it.
///
/// It's also the struct that does the deserializing.
/// It's called "DeserializerArgs", not "Deserializer". Why?
/// Because "args" makes more sense to the caller of [`ComponentBody::deserialize`], which
/// takes an [`DeserializerArgs`] argument. However, you can think of "DeserializerArgs" as the deserializer.
pub trait DeserializerArgs<TOutput>: Clone
where
    TOutput: ToTokens,
{
    // There's a lot of Results out there... let's make sure that this is a syn::Result.
    // Let's also make sure there's not a warning.
    /// Creates a [`ToTokens`] struct from the `self` args and a [`ComponentBody`].
    /// The [`ComponentBody::deserialize`] provides a cleaner way of calling this function.
    #[allow(unused_qualifications)]
    fn to_output(&self, component_body: &ComponentBody) -> syn::Result<TOutput>;
}
