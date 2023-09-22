use crate::component_body::{ComponentBody, DeserializerArgs};
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::token::Comma;
use syn::{punctuated::Punctuated, *};

/// The args and deserializing implementation for the [`crate::inline_props`] macro.
#[derive(Clone)]
pub struct InlinePropsDeserializerArgs;

/// The output fields and [`ToTokens`] implementation for the [`crate::inline_props`] macro.
#[derive(Clone)]
pub struct InlinePropsDeserializerOutput {
    pub comp_fn: ItemFn,
    pub props_struct: ItemStruct,
}

impl ToTokens for InlinePropsDeserializerOutput {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let function = &self.comp_fn;
        let props_struct = &self.props_struct;

        tokens.append_all(quote! {
            #function
            #props_struct
        });
    }
}

impl DeserializerArgs<InlinePropsDeserializerOutput> for InlinePropsDeserializerArgs {
    fn to_output(&self, component_body: &ComponentBody) -> Result<InlinePropsDeserializerOutput> {
        Ok(InlinePropsDeserializerOutput {
            comp_fn: Self::get_function(component_body),
            props_struct: Self::get_props_struct(component_body),
        })
    }
}

impl InlinePropsDeserializerArgs {
    fn get_props_struct(component_body: &ComponentBody) -> ItemStruct {
        let ComponentBody { item_fn, .. } = component_body;
        let ItemFn { vis, sig, .. } = item_fn;
        let Signature {
            inputs,
            ident: fn_ident,
            generics,
            ..
        } = sig;

        // Skip first arg since that's the context
        let struct_fields = inputs.iter().skip(1).map(move |f| {
            match f {
                FnArg::Receiver(_) => unreachable!(), // Unreachable because of ComponentBody parsing
                FnArg::Typed(pt) => {
                    let arg_pat = &pt.pat; // Pattern (identifier)
                    let arg_colon = &pt.colon_token;
                    let arg_ty = &pt.ty; // Type
                    let arg_attrs = &pt.attrs; // Attributes

                    quote! {
                        #(#arg_attrs)
                        *
                        #vis #arg_pat #arg_colon #arg_ty
                    }
                }
            }
        });

        let struct_ident = Ident::new(&format!("{fn_ident}Props"), fn_ident.span());

        let first_lifetime = if let Some(GenericParam::Lifetime(lt)) = generics.params.first() {
            Some(lt)
        } else {
            None
        };

        let struct_attrs = if first_lifetime.is_some() {
            quote! { #[derive(Props)] }
        } else {
            quote! { #[derive(Props, PartialEq)] }
        };

        let struct_generics = if first_lifetime.is_some() {
            let struct_generics: Punctuated<GenericParam, Comma> = component_body
                .item_fn
                .sig
                .generics
                .params
                .iter()
                .map(|it| match it {
                    GenericParam::Type(tp) => {
                        let mut tp = tp.clone();
                        tp.bounds.push(parse_quote!( 'a ));

                        GenericParam::Type(tp)
                    }
                    _ => it.clone(),
                })
                .collect();

            quote! { <#struct_generics> }
        } else {
            quote! { #generics }
        };

        parse_quote! {
            #struct_attrs
            #[allow(non_camel_case_types)]
            #vis struct #struct_ident #struct_generics
            {
                #(#struct_fields),*
            }
        }
    }

    fn get_function(component_body: &ComponentBody) -> ItemFn {
        let ComponentBody {
            item_fn,
            cx_pat_type,
            ..
        } = component_body;
        let ItemFn {
            attrs: fn_attrs,
            vis,
            sig,
            block: fn_block,
        } = item_fn;
        let Signature {
            inputs,
            ident: fn_ident,
            generics,
            output: fn_output,
            asyncness,
            ..
        } = sig;
        let Generics { where_clause, .. } = generics;

        let cx_pat = &cx_pat_type.pat;
        let struct_ident = Ident::new(&format!("{fn_ident}Props"), fn_ident.span());

        // Skip first arg since that's the context
        let struct_field_names = inputs.iter().skip(1).filter_map(|f| match f {
            FnArg::Receiver(_) => unreachable!(), // ComponentBody prohibits receiver parameters.
            FnArg::Typed(t) => Some(&t.pat),
        });

        let first_lifetime = if let Some(GenericParam::Lifetime(lt)) = generics.params.first() {
            Some(lt)
        } else {
            None
        };

        let (scope_lifetime, fn_generics) = if let Some(lt) = first_lifetime {
            (quote! { #lt, }, generics.clone())
        } else {
            let lifetime: LifetimeParam = parse_quote! { 'a };

            let mut fn_generics = generics.clone();
            fn_generics
                .params
                .insert(0, GenericParam::Lifetime(lifetime.clone()));

            (quote! { #lifetime, }, fn_generics)
        };

        let generics_no_bounds = {
            let mut generics = generics.clone();
            generics.params = generics
                .params
                .iter()
                .map(|it| match it {
                    GenericParam::Type(tp) => {
                        let mut tp = tp.clone();
                        tp.bounds.clear();

                        GenericParam::Type(tp)
                    }
                    _ => it.clone(),
                })
                .collect();

            generics
        };

        parse_quote! {
            #(#fn_attrs)*
            #asyncness #vis fn #fn_ident #fn_generics (#cx_pat: Scope<#scope_lifetime #struct_ident #generics_no_bounds>) #fn_output
            #where_clause
            {
                let #struct_ident { #(#struct_field_names),* } = &#cx_pat.props;
                #fn_block
            }
        }
    }
}
