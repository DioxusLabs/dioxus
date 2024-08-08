use proc_macro2::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::*;

pub struct ComponentBody {
    pub item_fn: ItemFn,
}

impl Parse for ComponentBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let item_fn: ItemFn = input.parse()?;
        validate_component_fn(&item_fn)?;
        Ok(Self { item_fn })
    }
}

impl ToTokens for ComponentBody {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        // https://github.com/DioxusLabs/dioxus/issues/1938
        // If there's only one input and the input is `props: Props`, we don't need to generate a props struct
        // Just attach the non_snake_case attribute to the function
        // eventually we'll dump this metadata into devtooling that lets us find all these components
        //
        // Components can also use the struct pattern to "inline" their props.
        // Freya uses this a bunch (because it's clean),
        // e.g. `fn Navbar(NavbarProps { title }: NavbarProps)` was previously being incorrectly parsed
        if self.is_explicit_props_ident() || self.has_struct_parameter_pattern() {
            let comp_fn = &self.item_fn;
            tokens.append_all(allow_camel_case_for_fn_ident(comp_fn).into_token_stream());
            return;
        }

        let comp_fn = self.comp_fn();

        // If there's no props declared, we simply omit the props argument
        // This is basically so you can annotate the App component with #[component] and still be compatible with the
        // launch signatures that take fn() -> Element
        let props_struct = match self.item_fn.sig.inputs.is_empty() {
            // No props declared, so we don't need to generate a props struct
            true => quote! {},

            // Props declared, so we generate a props struct and then also attach the doc attributes to it
            false => {
                let doc = format!("Properties for the [`{}`] component.", &comp_fn.sig.ident);
                let props_struct = self.props_struct();
                quote! {
                    #[doc = #doc]
                    #props_struct
                }
            }
        };

        let completion_hints = self.completion_hints();

        tokens.append_all(quote! {
            #props_struct
            #comp_fn

            #completion_hints
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
            ..
        } = sig;

        let Generics { where_clause, .. } = generics;
        let (_, impl_generics, _) = generics.split_for_impl();
        let generics_turbofish = impl_generics.as_turbofish();

        // We generate a struct with the same name as the component but called `Props`
        let struct_ident = Ident::new(&format!("{fn_ident}Props"), fn_ident.span());

        // We pull in the field names from the original function signature, but need to strip off the mutability
        let struct_field_names = inputs.iter().map(rebind_mutability);

        let props_docs = self.props_docs(inputs.iter().collect());

        let inlined_props_argument = if inputs.is_empty() {
            quote! {}
        } else {
            quote! { #struct_ident { #(#struct_field_names),* }: #struct_ident #impl_generics }
        };

        // The extra nest is for the snake case warning to kick back in
        parse_quote! {
            #(#attrs)*
            #(#props_docs)*
            #[allow(non_snake_case)]
            #vis fn #fn_ident #generics (#inlined_props_argument) #fn_output #where_clause {
                {
                    // In debug mode we can detect if the user is calling the component like a function
                    dioxus_core::internal::verify_component_called_as_component(#fn_ident #generics_turbofish);
                    #block
                }
            }
        }
    }

    /// Build an associated struct for the props of the component
    ///
    /// This will expand to the typed-builder implementation that we have vendored in this crate.
    /// TODO: don't vendor typed-builder and instead transform the tokens we give it before expansion.
    /// TODO: cache these tokens since this codegen is rather expensive (lots of tokens)
    ///
    /// We try our best to transfer over any declared doc attributes from the original function signature onto the
    /// props struct fields.
    fn props_struct(&self) -> ItemStruct {
        let ItemFn { vis, sig, .. } = &self.item_fn;
        let Signature {
            inputs,
            ident,
            generics,
            ..
        } = sig;

        let struct_fields = inputs.iter().map(move |f| make_prop_struct_field(f, vis));
        let struct_ident = Ident::new(&format!("{ident}Props"), ident.span());

        parse_quote! {
            #[derive(Props, Clone, PartialEq)]
            #[allow(non_camel_case_types)]
            #vis struct #struct_ident #generics {
                #(#struct_fields),*
            }
        }
    }

    /// Convert a list of function arguments into a list of doc attributes for the props struct
    ///
    /// This lets us generate set of attributes that we can apply to the props struct to give it a nice docstring.
    fn props_docs(&self, inputs: Vec<&FnArg>) -> Vec<Attribute> {
        let fn_ident = &self.item_fn.sig.ident;

        if inputs.is_empty() {
            return Vec::new();
        }

        let arg_docs = inputs
            .iter()
            .filter_map(|f| build_doc_fields(f))
            .collect::<Vec<_>>();

        let mut props_docs = Vec::with_capacity(5);
        let props_def_link = fn_ident.to_string() + "Props";
        let header =
            format!("# Props\n*For details, see the [props struct definition]({props_def_link}).*");

        props_docs.push(parse_quote! {
            #[doc = #header]
        });

        for arg in arg_docs {
            let DocField {
                arg_name,
                arg_type,
                deprecation,
                input_arg_doc,
            } = arg;

            let arg_name = strip_pat_mutability(arg_name).to_token_stream().to_string();
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

            props_docs.push(parse_quote! { #[doc = #arg_doc] });
        }

        props_docs
    }

    fn is_explicit_props_ident(&self) -> bool {
        if let Some(FnArg::Typed(PatType { pat, .. })) = self.item_fn.sig.inputs.first() {
            if let Pat::Ident(ident) = pat.as_ref() {
                return ident.ident == "props";
            }
        }

        false
    }

    fn has_struct_parameter_pattern(&self) -> bool {
        if let Some(FnArg::Typed(PatType { pat, .. })) = self.item_fn.sig.inputs.first() {
            if matches!(pat.as_ref(), Pat::Struct(_)) {
                return true;
            }
        }

        false
    }

    // We generate an extra enum to help us autocomplete the braces after the component.
    // This is a bit of a hack, but it's the only way to get the braces to autocomplete.
    fn completion_hints(&self) -> TokenStream {
        let comp_fn = &self.item_fn.sig.ident;
        let completions_mod = Ident::new(&format!("{}_completions", comp_fn), comp_fn.span());

        let vis = &self.item_fn.vis;

        quote! {
            #[allow(non_snake_case)]
            #[doc(hidden)]
            mod #completions_mod {
                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                /// This enum is generated to help autocomplete the braces after the component. It does nothing
                pub enum Component {
                    #comp_fn {}
                }
            }

            #[allow(unused)]
            #vis use #completions_mod::Component::#comp_fn;
        }
    }
}

struct DocField<'a> {
    arg_name: &'a Pat,
    arg_type: &'a Type,
    deprecation: Option<crate::utils::DeprecatedAttribute>,
    input_arg_doc: String,
}

fn build_doc_fields(f: &FnArg) -> Option<DocField> {
    let FnArg::Typed(pt) = f else { unreachable!() };

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

    Some(DocField {
        arg_name: &pt.pat,
        arg_type: &pt.ty,
        deprecation: pt.attrs.iter().find_map(|attr| {
            if !attr.path().is_ident("deprecated") {
                return None;
            }

            let res = crate::utils::DeprecatedAttribute::from_meta(&attr.meta);

            match res {
                Err(e) => panic!("{}", e.to_string()),
                Ok(v) => Some(v),
            }
        }),
        input_arg_doc: arg_doc,
    })
}

fn validate_component_fn(item_fn: &ItemFn) -> Result<()> {
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

/// Convert a function arg with a given visibility (provided by the function) and then generate a field for the
/// associated props struct.
fn make_prop_struct_field(f: &FnArg, vis: &Visibility) -> TokenStream {
    // There's no receivers (&self) allowed in the component body
    let FnArg::Typed(pt) = f else { unreachable!() };

    let arg_pat = match pt.pat.as_ref() {
        // rip off mutability
        // todo: we actually don't want any of the extra bits of the field pattern
        Pat::Ident(f) => {
            let mut f = f.clone();
            f.mutability = None;
            quote! { #f }
        }
        a => quote! { #a },
    };

    let PatType {
        attrs,
        ty,
        colon_token,
        ..
    } = pt;

    quote! {
        #(#attrs)*
        #vis #arg_pat #colon_token #ty
    }
}

fn rebind_mutability(f: &FnArg) -> TokenStream {
    // There's no receivers (&self) allowed in the component body
    let FnArg::Typed(pt) = f else { unreachable!() };

    let immutable = strip_pat_mutability(&pt.pat);

    quote!(mut #immutable)
}

fn strip_pat_mutability(pat: &Pat) -> Pat {
    let mut pat = pat.clone();
    // rip off mutability, but still write it out eventually
    if let Pat::Ident(ref mut pat_ident) = &mut pat {
        pat_ident.mutability = None;
    }

    pat
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

/// Takes a function and returns a clone of it where an `UpperCamelCase` identifier is allowed by the compiler.
fn allow_camel_case_for_fn_ident(item_fn: &ItemFn) -> ItemFn {
    let mut clone = item_fn.clone();
    let block = &item_fn.block;

    clone.attrs.push(parse_quote! { #[allow(non_snake_case)] });

    clone.block = parse_quote! {
        {
            #block
        }
    };

    clone
}
