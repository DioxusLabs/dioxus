use proc_macro::TokenStream;

use convert_case::{Case, Casing};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::__private::TokenStream2;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, parse_macro_input, Ident, Token};

#[proc_macro]
pub fn impl_extension_attributes(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ImplExtensionAttributes);
    input.to_token_stream().into()
}

struct ImplExtensionAttributes {
    is_element: bool,
    name: Ident,
    attrs: Punctuated<Ident, Token![,]>,
}

impl Parse for ImplExtensionAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;

        let element: Ident = input.parse()?;
        let name = input.parse()?;
        braced!(content in input);
        let attrs = content.parse_terminated(Ident::parse, Token![,])?;

        Ok(ImplExtensionAttributes {
            is_element: element == "ELEMENT",
            name,
            attrs,
        })
    }
}

impl ToTokens for ImplExtensionAttributes {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;
        let name_string = name.to_string();
        let camel_name = name_string
            .strip_prefix("r#")
            .unwrap_or(&name_string)
            .to_case(Case::UpperCamel);
        let impl_name = Ident::new(format!("{}Impl", &camel_name).as_str(), name.span());
        let extension_name = Ident::new(format!("{}Extension", &camel_name).as_str(), name.span());

        if !self.is_element {
            tokens.append_all(quote! {
                struct #impl_name;
                impl #name for #impl_name {}
            });
        }

        let impls = self.attrs.iter().map(|ident| {
            let d = if self.is_element {
                quote! { #name::#ident }
            } else {
                quote! { <#impl_name as #name>::#ident }
            };
            quote! {
                fn #ident(self, value: impl IntoAttributeValue) -> Self {
                    let d = #d;
                    self.push_attribute(d.0, d.1, value, d.2)
                }
            }
        });
        tokens.append_all(quote! {
            pub trait #extension_name: HasAttributes + Sized {
                #(#impls)*
            }
        });
    }
}
