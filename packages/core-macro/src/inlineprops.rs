use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    *,
};

pub struct InlinePropsBody {
    pub attrs: Vec<Attribute>,
    pub vis: syn::Visibility,
    pub maybe_async: Option<Token![async]>,
    pub fn_token: Token![fn],
    pub ident: Ident,
    pub cx_token: Box<Pat>,
    pub generics: Generics,
    pub paren_token: token::Paren,
    pub inputs: Punctuated<FnArg, Token![,]>,
    // pub fields: FieldsNamed,
    pub output: ReturnType,
    pub where_clause: Option<WhereClause>,
    pub block: Box<Block>,
}

/// The custom rusty variant of parsing rsx!
impl Parse for InlinePropsBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs: Vec<Attribute> = input.call(Attribute::parse_outer)?;
        let maybe_async: Option<Token![async]> = input.parse().ok();
        let vis: Visibility = input.parse()?;

        let fn_token = input.parse()?;
        let ident = input.parse()?;
        let generics: Generics = input.parse()?;

        let content;
        let paren_token = syn::parenthesized!(content in input);

        let first_arg: FnArg = content.parse()?;
        let cx_token = {
            match first_arg {
                FnArg::Receiver(_) => panic!("first argument must not be a receiver argument"),
                FnArg::Typed(f) => f.pat,
            }
        };

        let _: Result<Token![,]> = content.parse();

        let inputs = syn::punctuated::Punctuated::parse_terminated(&content)?;

        let output = input.parse()?;

        let where_clause = input
            .peek(syn::token::Where)
            .then(|| input.parse())
            .transpose()?;

        let block = input.parse()?;

        Ok(Self {
            vis,
            maybe_async,
            fn_token,
            ident,
            generics,
            paren_token,
            inputs,
            output,
            where_clause,
            block,
            cx_token,
            attrs,
        })
    }
}

/// Serialize the same way, regardless of flavor
impl ToTokens for InlinePropsBody {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let Self {
            vis,
            ident,
            generics,
            inputs,
            output,
            where_clause,
            block,
            cx_token,
            attrs,
            maybe_async,
            ..
        } = self;

        let fields = inputs.iter().map(|f| {
            quote! { #vis #f }
        });

        let struct_name = Ident::new(&format!("{ident}Props"), Span::call_site());

        let field_names = inputs.iter().filter_map(|f| match f {
            FnArg::Receiver(_) => todo!(),
            FnArg::Typed(t) => Some(&t.pat),
        });

        let first_lifetime = if let Some(GenericParam::Lifetime(lt)) = generics.params.first() {
            Some(lt)
        } else {
            None
        };

        let modifiers = if first_lifetime.is_some() {
            quote! { #[derive(Props)] }
        } else {
            quote! { #[derive(Props, PartialEq)] }
        };

        let (scope_lifetime, fn_generics, struct_generics) = if let Some(lt) = first_lifetime {
            let struct_generics: Punctuated<_, token::Comma> = generics
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

            (
                quote! { #lt, },
                generics.clone(),
                quote! { <#struct_generics> },
            )
        } else {
            let lifetime: LifetimeDef = parse_quote! { 'a };

            let mut fn_generics = generics.clone();
            fn_generics
                .params
                .insert(0, GenericParam::Lifetime(lifetime.clone()));

            (quote! { #lifetime, }, fn_generics, quote! { #generics })
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

        out_tokens.append_all(quote! {
            #modifiers
            #[allow(non_camel_case_types)]
            #vis struct #struct_name #struct_generics
            #where_clause
            {
                #(#fields),*
            }

            #(#attrs)*
            #maybe_async #vis fn #ident #fn_generics (#cx_token: Scope<#scope_lifetime #struct_name #generics_no_bounds>) #output
            #where_clause
            {
                let #struct_name { #(#field_names),* } = &#cx_token.props;
                #block
            }
        });
    }
}
