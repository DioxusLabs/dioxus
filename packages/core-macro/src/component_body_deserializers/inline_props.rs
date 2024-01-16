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
            comp_fn: get_function(component_body),
            props_struct: get_props_struct(component_body),
        })
    }
}

fn get_props_struct(component_body: &ComponentBody) -> ItemStruct {
    let ComponentBody { item_fn, .. } = component_body;
    let ItemFn { vis, sig, .. } = item_fn;
    let Signature {
        inputs,
        ident: fn_ident,
        generics,
        ..
    } = sig;

    let struct_fields = inputs.iter().map(move |f| {
        match f {
            FnArg::Receiver(_) => unreachable!(), // Unreachable because of ComponentBody parsing
            FnArg::Typed(pt) => {
                let arg_pat = match pt.pat.as_ref() {
                    // rip off mutability
                    Pat::Ident(f) => {
                        let mut f = f.clone();
                        f.mutability = None;
                        quote! { #f }
                    }
                    a => quote! { #a },
                };

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
        quote! { #[derive(Props, Clone)] }
    } else {
        quote! { #[derive(Props, Clone, PartialEq)] }
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

fn get_props_docs(fn_ident: &Ident, inputs: Vec<&FnArg>) -> Vec<Attribute> {
    if inputs.len() <= 1 {
        return Vec::new();
    }

    let arg_docs = inputs
        .iter()
        .filter_map(|f| match f {
            FnArg::Receiver(_) => unreachable!(), // ComponentBody prohibits receiver parameters.
            FnArg::Typed(pt) => {
                let arg_doc = pt
                    .attrs
                    .iter()
                    .filter_map(|attr| {
                        // TODO: Error reporting
                        // Check if the path of the attribute is "doc"
                        if !is_attr_doc(attr) {
                            return None;
                        };

                        let Meta::NameValue(meta_name_value) = &attr.meta else {
                            return None;
                        };

                        let Expr::Lit(doc_lit) = &meta_name_value.value else {
                            return None;
                        };

                        let Lit::Str(doc_lit_str) = &doc_lit.lit else {
                            return None;
                        };

                        Some(doc_lit_str.value())
                    })
                    .fold(String::new(), |mut doc, next_doc_line| {
                        doc.push('\n');
                        doc.push_str(&next_doc_line);
                        doc
                    });

                Some((
                    &pt.pat,
                    &pt.ty,
                    pt.attrs.iter().find_map(|attr| {
                        if attr.path() != &parse_quote!(deprecated) {
                            return None;
                        }

                        let res = crate::utils::DeprecatedAttribute::from_meta(&attr.meta);

                        match res {
                            Err(e) => panic!("{}", e.to_string()),
                            Ok(v) => Some(v),
                        }
                    }),
                    arg_doc,
                ))
            }
        })
        .collect::<Vec<_>>();

    let mut props_docs = Vec::with_capacity(5);
    let props_def_link = fn_ident.to_string() + "Props";
    let header =
        format!("# Props\n*For details, see the [props struct definition]({props_def_link}).*");

    props_docs.push(parse_quote! {
        #[doc = #header]
    });

    for (arg_name, arg_type, deprecation, input_arg_doc) in arg_docs {
        let arg_name = arg_name.into_token_stream().to_string();
        let arg_type = crate::utils::format_type_string(arg_type);

        let input_arg_doc = keep_up_to_n_consecutive_chars(input_arg_doc.trim(), 2, '\n')
            .replace("\n\n", "</p><p>");
        let prop_def_link = format!("{props_def_link}::{arg_name}");
        let mut arg_doc = format!("- [`{arg_name}`]({prop_def_link}) : `{arg_type}`");

        if let Some(deprecation) = deprecation {
            arg_doc.push_str("<p>👎 Deprecated");

            if let Some(since) = deprecation.since {
                arg_doc.push_str(&format!(" since {since}"));
            }

            if let Some(note) = deprecation.note {
                let note = keep_up_to_n_consecutive_chars(&note, 1, '\n').replace('\n', " ");
                let note = keep_up_to_n_consecutive_chars(&note, 1, '\t').replace('\t', " ");

                arg_doc.push_str(&format!(": {note}"));
            }

            arg_doc.push_str("</p>");

            if !input_arg_doc.is_empty() {
                arg_doc.push_str("<hr/>");
            }
        }

        if !input_arg_doc.is_empty() {
            arg_doc.push_str(&format!("<p>{input_arg_doc}</p>"));
        }

        props_docs.push(parse_quote! {
            #[doc = #arg_doc]
        });
    }

    props_docs
}

fn get_function(component_body: &ComponentBody) -> ItemFn {
    let ComponentBody { item_fn, .. } = component_body;
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

    let struct_ident = Ident::new(&format!("{fn_ident}Props"), fn_ident.span());

    // Skip first arg since that's the context
    let struct_field_names = inputs.iter().filter_map(|f| match f {
        FnArg::Receiver(_) => unreachable!(), // ComponentBody prohibits receiver parameters.
        FnArg::Typed(pt) => {
            let pat = &pt.pat;

            let mut pat = pat.clone();

            // rip off mutability, but still write it out eventually
            if let Pat::Ident(ref mut pat_ident) = pat.as_mut() {
                pat_ident.mutability = None;
            }

            Some(quote!(mut  #pat))
        }
    });

    let first_lifetime = if let Some(GenericParam::Lifetime(lt)) = generics.params.first() {
        Some(lt)
    } else {
        None
    };

    let (_scope_lifetime, fn_generics) = if let Some(lt) = first_lifetime {
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

    let props_docs = get_props_docs(fn_ident, inputs.iter().skip(1).collect());

    parse_quote! {
        #(#fn_attrs)*
        #(#props_docs)*
        #asyncness #vis fn #fn_ident #fn_generics (mut __props: #struct_ident #generics_no_bounds) #fn_output
        #where_clause
        {
            let #struct_ident { #(#struct_field_names),* } = __props;
            #fn_block
        }
    }
}

/// Checks if the attribute is a `#[doc]` attribute.
fn is_attr_doc(attr: &Attribute) -> bool {
    attr.path() == &parse_quote!(doc)
}

fn keep_up_to_n_consecutive_chars(
    input: &str,
    n_of_consecutive_chars_allowed: usize,
    target_char: char,
) -> String {
    let mut output = String::new();
    let mut prev_char: Option<char> = None;
    let mut consecutive_count = 0;

    for c in input.chars() {
        match prev_char {
            Some(prev) if c == target_char && prev == target_char => {
                if consecutive_count < n_of_consecutive_chars_allowed {
                    output.push(c);
                    consecutive_count += 1;
                }
            }
            _ => {
                output.push(c);
                prev_char = Some(c);
                consecutive_count = 1;
            }
        }
    }

    output
}
