use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token, Block, FnArg, Generics, Ident, Pat, Result, ReturnType, Token, Visibility,
};

pub struct InlinePropsBody {
    pub vis: syn::Visibility,
    pub fn_token: Token![fn],
    pub ident: Ident,
    pub cx_token: Box<Pat>,
    pub generics: Generics,
    pub paren_token: token::Paren,
    pub inputs: Punctuated<FnArg, Token![,]>,
    // pub fields: FieldsNamed,
    pub output: ReturnType,
    pub block: Box<Block>,
}

/// The custom rusty variant of parsing rsx!
impl Parse for InlinePropsBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let vis: Visibility = input.parse()?;

        let fn_token = input.parse()?;
        let ident = input.parse()?;
        let generics = input.parse()?;

        let content;
        let paren_token = syn::parenthesized!(content in input);

        let first_arg: FnArg = content.parse()?;
        let cx_token = {
            match first_arg {
                FnArg::Receiver(_) => panic!("first argument must not be  a reciver argument"),
                FnArg::Typed(f) => f.pat,
            }
        };

        let _: Result<Token![,]> = content.parse();

        let inputs = syn::punctuated::Punctuated::parse_terminated(&content)?;

        let output = input.parse()?;

        let block = input.parse()?;

        Ok(Self {
            vis,
            fn_token,
            ident,
            generics,
            paren_token,
            inputs,
            output,
            block,
            cx_token,
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
            block,
            cx_token,
            ..
        } = self;

        let fields = inputs.iter().map(|f| {
            quote! { #vis #f }
        });

        let struct_name = Ident::new(&format!("{}Props", ident), Span::call_site());

        let field_names = inputs.iter().filter_map(|f| match f {
            FnArg::Receiver(_) => todo!(),
            FnArg::Typed(t) => Some(&t.pat),
        });

        let modifiers = if generics.params.is_empty() {
            quote! { #[derive(Props, PartialEq)] }
        } else {
            quote! { #[derive(Props)] }
        };

        out_tokens.append_all(quote! {
            #modifiers
            #vis struct #struct_name #generics {
                #(#fields),*
            }

            #vis fn #ident #generics (#cx_token: Scope<'a, #struct_name #generics>) #output {
                let #struct_name { #(#field_names),* } = &cx.props;
                #block
            }
        });
    }
}
