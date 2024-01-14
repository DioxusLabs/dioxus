use crate::component_body::{ComponentBody, DeserializerArgs};
use crate::component_body_deserializers::inline_props::InlinePropsDeserializerArgs;
use constcat::concat;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::*;

pub(crate) const COMPONENT_ARG_CASE_CHECK_ERROR: &str = concat!(
    "This component does not use PascalCase. \
To ignore this check, pass the \"",
    crate::COMPONENT_ARG_CASE_CHECK_OFF,
    "\" argument, like so: #[component(",
    crate::COMPONENT_ARG_CASE_CHECK_OFF,
    ")]"
);

const INNER_FN_NAME: &str = "__dx_inner_comp";

fn get_out_comp_fn(orig_comp_fn: &ItemFn) -> ItemFn {
    let inner_comp_ident = Ident::new(INNER_FN_NAME, orig_comp_fn.sig.ident.span());

    let inner_comp_fn = ItemFn {
        sig: Signature {
            ident: inner_comp_ident.clone(),
            ..orig_comp_fn.sig.clone()
        },
        ..orig_comp_fn.clone()
    };

    let props_ident = match orig_comp_fn.sig.inputs.is_empty() {
        true => quote! {},
        false => quote! { __props },
    };

    ItemFn {
        block: parse_quote! {
            {
                #[warn(non_snake_case)]
                #[allow(clippy::inline_always)]
                #[inline(always)]
                #inner_comp_fn
                #inner_comp_ident(#props_ident)
            }
        },
        ..orig_comp_fn.clone()
    }
}

/// The args and deserializing implementation for the [`crate::component`] macro.
#[derive(Clone)]
pub struct ComponentDeserializerArgs {
    pub case_check: bool,
}

/// The output fields and [`ToTokens`] implementation for the [`crate::component`] macro.
#[derive(Clone)]
pub struct ComponentDeserializerOutput {
    pub comp_fn: ItemFn,
    pub props_struct: Option<ItemStruct>,
}

impl ToTokens for ComponentDeserializerOutput {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let comp_fn = &self.comp_fn;
        let props_struct = &self.props_struct;
        let fn_ident = &comp_fn.sig.ident;

        let doc = format!("Properties for the [`{fn_ident}`] component.");
        tokens.append_all(quote! {
            #[doc = #doc]
            #props_struct
            #[allow(non_snake_case)]
            #comp_fn
        });
    }
}

impl DeserializerArgs<ComponentDeserializerOutput> for ComponentDeserializerArgs {
    fn to_output(&self, component_body: &ComponentBody) -> Result<ComponentDeserializerOutput> {
        let Signature { ident, .. } = &component_body.item_fn.sig;

        if self.case_check && !is_pascal_case(&ident.to_string()) {
            return Err(Error::new(ident.span(), COMPONENT_ARG_CASE_CHECK_ERROR));
        }

        if component_body.has_extra_args {
            Self::deserialize_with_props(component_body)
        } else {
            Ok(Self::deserialize_no_props(component_body))
        }
    }
}

impl ComponentDeserializerArgs {
    fn deserialize_no_props(component_body: &ComponentBody) -> ComponentDeserializerOutput {
        let ComponentBody { item_fn, .. } = component_body;

        let comp_fn = get_out_comp_fn(item_fn);

        ComponentDeserializerOutput {
            comp_fn,
            props_struct: None,
        }
    }

    fn deserialize_with_props(
        component_body: &ComponentBody,
    ) -> Result<ComponentDeserializerOutput> {
        let ComponentBody { item_fn, .. } = component_body;

        let comp_parsed = match parse2::<ComponentBody>(quote!(#item_fn)) {
            Ok(comp_body) => comp_body,
            Err(e) => {
                return Err(Error::new(
                    e.span(),
                    format!(
                        "This is probably a bug in our code, please report it! Error: {}",
                        e
                    ),
                ))
            }
        };

        let inlined_props_output = comp_parsed.deserialize(InlinePropsDeserializerArgs {})?;
        let props_struct = inlined_props_output.props_struct;
        let props_fn = inlined_props_output.comp_fn;

        let comp_fn = get_out_comp_fn(&props_fn);

        Ok(ComponentDeserializerOutput {
            comp_fn,
            props_struct: Some(props_struct),
        })
    }
}

fn is_pascal_case(input: &str) -> bool {
    let mut is_next_lowercase = false;

    for c in input.chars() {
        let is_upper = c.is_ascii_uppercase();

        if (c == '_') || (is_upper && is_next_lowercase) {
            return false;
        }

        is_next_lowercase = is_upper;
    }

    true
}
