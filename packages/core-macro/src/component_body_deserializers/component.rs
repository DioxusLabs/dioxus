use crate::component_body::{ComponentBody, DeserializerArgs, DeserializerOutput};
use constcat::concat;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::*;
use crate::component_body_deserializers::inline_props::InlinePropsDeserializerArgs;

pub(crate) const COMPONENT_ARG_CASE_CHECK_ERROR: &str = concat!(
    "This component does not use snake_case. \
To ignore this check and prevent converting the name to PascalCase, pass the \"",
    crate::COMPONENT_ARG_CASE_CHECK_OFF,
    "\" argument, like so: #[component(",
    crate::COMPONENT_ARG_CASE_CHECK_OFF,
    ")]"
);

/// The args and deserializing implementation for the [`crate::component`] macro.
#[derive(Clone)]
pub struct ComponentDeserializerArgs {
    pub case_check: bool,
}

/// The output fields and [`ToTokens`] implementation for the [`crate::component`] macro.
#[derive(Clone)]
pub struct ComponentDeserializerOutput {
    pub comp_fn: ItemFn,
    pub hidden_comp_fn: ItemFn,
    pub props_struct: Option<ItemStruct>,
}

impl DeserializerOutput for ComponentDeserializerOutput {}

impl ToTokens for ComponentDeserializerOutput {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let comp_fn = &self.comp_fn;
        let hidden_comp_fn = &self.hidden_comp_fn;
        let props_struct = &self.props_struct;

        tokens.append_all(quote! {
            #props_struct
            #[allow(non_snake_case)]
            #comp_fn
            #[inline(always)]
            #hidden_comp_fn
        });
    }
}

impl DeserializerArgs<ComponentDeserializerOutput> for ComponentDeserializerArgs {
    fn to_output(&self, component_body: &ComponentBody) -> Result<ComponentDeserializerOutput> {
        let Signature { ident, .. } = &component_body.item_fn.sig;

        if self.case_check && !is_snake_case(&*ident.to_string()) {
            return Err(Error::new(ident.span(), COMPONENT_ARG_CASE_CHECK_ERROR));
        }

        return if component_body.has_extra_args {
            Self::deserialize_with_props(component_body)
        } else {
            Ok(Self::deserialize_no_props(component_body))
        };
    }
}

impl ComponentDeserializerArgs {
    fn deserialize_no_props(component_body: &ComponentBody) -> ComponentDeserializerOutput {
        let ComponentBody {
            item_fn,
            cx_pat_type,
            ..
        } = component_body;
        let ItemFn { sig, .. } = item_fn;
        let Signature { ident, .. } = sig;
        let cx_pat = &cx_pat_type.pat;

        let pascal_name = &*snake_to_pascal(&*ident.to_string());
        let comp_ident = Ident::new(pascal_name, ident.span());
        let hidden_comp_ident = Ident::new(&format!("__{ident}"), ident.span());

        let comp_sig = Signature {
            ident: comp_ident,
            ..sig.clone()
        };
        let comp_fn = ItemFn {
            sig: comp_sig,
            block: parse_quote!({#hidden_comp_ident (#cx_pat)}),
            ..item_fn.clone()
        };

        let hidden_comp_sig = Signature {
            ident: hidden_comp_ident,
            ..sig.clone()
        };
        let hidden_comp_fn = ItemFn {
            sig: hidden_comp_sig,
            ..item_fn.clone()
        };

        ComponentDeserializerOutput {
            comp_fn,
            hidden_comp_fn,
            props_struct: None,
        }
    }

    fn deserialize_with_props(component_body: &ComponentBody) -> Result<ComponentDeserializerOutput> {
        let ComponentBody {
            item_fn,
            cx_pat_type,
            ..
        } = component_body;
        let ItemFn { sig, .. } = item_fn;
        let Signature { ident, .. } = sig;
        let cx_pat = &cx_pat_type.pat;

        let pascal_name = &*snake_to_pascal(&*ident.to_string());
        let comp_ident = Ident::new(pascal_name, ident.span());
        let hidden_comp_ident = Ident::new(&format!("__{ident}"), ident.span());

        let comp_fn = ItemFn {
            sig: Signature {
                ident: comp_ident,
                ..sig.clone()
            },
            ..item_fn.clone()
        };
        let comp_parsed = match parse2::<ComponentBody>(quote!(#comp_fn)) {
            Ok(comp_body) => comp_body,
            Err(e) => {
                return Err(Error::new(
                    e.span(),
                    format!(
                        "This is probably a bug in our code, please report it! Error: {}",
                        e.to_string()
                    ),
                ))
            }
        };

        let inlined_props_output = comp_parsed.deserialize(InlinePropsDeserializerArgs {})?;
        let props_struct = inlined_props_output.props_struct;
        let props_fn = inlined_props_output.comp_fn;

        let hidden_comp_fn = ItemFn {
            sig: Signature {
                ident: hidden_comp_ident.clone(),
                ..props_fn.sig.clone()
            },
            ..props_fn.clone()
        };

        let comp_fn = ItemFn {
            block: parse_quote!({#hidden_comp_ident (#cx_pat)}),
            ..props_fn
        };

        Ok(ComponentDeserializerOutput {
            comp_fn,
            hidden_comp_fn,
            props_struct: Some(props_struct),
        })
    }
}

fn is_snake_case(input: &str) -> bool {
    let mut last_char: char = '0';

    // Skip initial underscores, repeated or not. These are still snake_case (at least in Rust).
    for c in input.chars().skip_while(|c| *c == '_') {
        // It's not snake_case, if:
        // - The char is an uppercase letter.
        // - If the char is a repeated underscore (e.g. "foo__bar").
        // Not sure how the alphabetic checking works for other languages though.
        if (c.is_alphabetic() && c.is_ascii_uppercase()) || (c == '_' && last_char == '_') {
            return false;
        }

        last_char = c;
    }

    true
}

fn snake_to_pascal(input: &str) -> String {
    let mut pascal = String::with_capacity(input.len());
    let mut capitalize_next = true;

    for c in input.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            pascal.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            pascal.push(c);
        }
    }

    pascal
}
