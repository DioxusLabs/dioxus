use proc_macro2::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::*;

/// General struct for parsing a component body.
/// However, because it's ambiguous, it does not implement [`ToTokens`](quote::to_tokens::ToTokens).
///
/// Refer to the [module documentation](crate::component_body) for more.
pub struct ComponentBody {
    pub item_fn: ItemFn,
}

impl Parse for ComponentBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let item_fn: ItemFn = input.parse()?;
        validate_component_fn_signature(&item_fn)?;
        Ok(Self { item_fn })
    }
}

impl ToTokens for ComponentBody {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let comp_fn = self.comp_fn();

        let props_struct = match self.item_fn.sig.inputs.is_empty() {
            true => quote! {},
            false => {
                let doc = format!("Properties for the [`{}`] component.", &comp_fn.sig.ident);
                let props_struct = self.props_struct();
                quote! {
                    #[doc = #doc]
                    #props_struct
                }
            }
        };

        tokens.append_all(quote! {
            #props_struct

            #[allow(non_snake_case)]
            #comp_fn
        });
    }
}

impl ComponentBody {
    // build a new item fn, transforming the original item fn
    fn comp_fn(&self) -> ItemFn {
        let ComponentBody { item_fn, .. } = self;
        let ItemFn {
            attrs,
            vis,
            sig,
            block,
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

        // We generate a struct with the same name as the component but called `Props`
        let struct_ident = Ident::new(&format!("{fn_ident}Props"), fn_ident.span());

        let struct_field_names = inputs.iter().filter_map(strip_mutability);
        let (_, ty_generics, _) = generics.split_for_impl();
        let props_docs = self.props_docs(inputs.iter().skip(1).collect());

        let props_ident = match inputs.is_empty() {
            true => quote! {},
            false => quote! { mut __props: #struct_ident #ty_generics },
        };

        let expanded_struct = match inputs.is_empty() {
            true => quote! {},
            false => quote! { let #struct_ident { #(#struct_field_names),* } = __props; },
        };

        parse_quote! {
            #(#attrs)*
            #(#props_docs)*
            #asyncness #vis fn #fn_ident #generics (#props_ident) #fn_output #where_clause {
                #expanded_struct
                #block
            }
        }
    }

    // Build the props struct
    fn props_struct(&self) -> ItemStruct {
        let ComponentBody { item_fn, .. } = &self;
        let ItemFn { vis, sig, .. } = item_fn;
        let Signature {
            inputs,
            ident,
            generics,
            ..
        } = sig;

        let struct_fields = inputs.iter().map(move |f| make_prop_struct_fields(f, vis));
        let struct_ident = Ident::new(&format!("{ident}Props"), ident.span());

        parse_quote! {
            #[derive(Props, Clone, PartialEq)]
            #[allow(non_camel_case_types)]
            #vis struct #struct_ident #generics
            { #(#struct_fields),* }
        }
    }

    fn props_docs(&self, inputs: Vec<&FnArg>) -> Vec<Attribute> {
        let fn_ident = &self.item_fn.sig.ident;

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
}

fn make_prop_struct_fields(f: &FnArg, vis: &Visibility) -> TokenStream {
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
}

fn strip_mutability(f: &FnArg) -> Option<TokenStream> {
    match f {
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

fn validate_component_fn_signature(item_fn: &ItemFn) -> Result<()> {
    // Do some validation....
    // 1. Ensure the component returns *something*
    if item_fn.sig.output == ReturnType::Default {
        return Err(Error::new(
            item_fn.sig.output.span(),
            "Must return a <dioxus_core::Element>".to_string(),
        ));
    }

    // 2. make sure there's no lifetimes on the component - we don't know how to handle those
    if item_fn.sig.generics.lifetimes().count() > 0 {
        return Err(Error::new(
            item_fn.sig.generics.span(),
            "Lifetimes are not supported in components".to_string(),
        ));
    }

    // 3. we can't handle async components
    if item_fn.sig.asyncness.is_some() {
        return Err(Error::new(
            item_fn.sig.asyncness.span(),
            "Async components are not supported".to_string(),
        ));
    }

    // 4. we can't handle const components
    if item_fn.sig.constness.is_some() {
        return Err(Error::new(
            item_fn.sig.constness.span(),
            "Const components are not supported".to_string(),
        ));
    }

    // 5. no receiver parameters
    if item_fn
        .sig
        .inputs
        .iter()
        .any(|f| matches!(f, FnArg::Receiver(_)))
    {
        return Err(Error::new(
            item_fn.sig.inputs.span(),
            "Receiver parameters are not supported".to_string(),
        ));
    }

    Ok(())
}
