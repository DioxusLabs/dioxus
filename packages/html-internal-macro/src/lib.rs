use proc_macro::TokenStream;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, TokenStreamExt, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, Token, braced, parse_macro_input};

#[proc_macro]
pub fn impl_extension_attributes(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ImplExtensionAttributes);
    input.to_token_stream().into()
}

struct ImplExtensionAttributes {
    name: Ident,
    attrs: Punctuated<Ident, Token![,]>,
    for_el: bool,
}

impl Parse for ImplExtensionAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;

        let name = input.parse()?;
        braced!(content in input);
        let attrs = content.parse_terminated(Ident::parse, Token![,])?;
        let for_el = if input.is_empty() {
            false
        } else {
            let marker: Ident = input.parse()?;
            if marker != "for_el" {
                return Err(syn::Error::new(
                    marker.span(),
                    "expected `for_el` after extension attribute list",
                ));
            }
            true
        };

        Ok(ImplExtensionAttributes {
            name,
            attrs,
            for_el,
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
        let extension_name = Ident::new(format!("{}Extension", &camel_name).as_str(), name.span());

        let descriptors = self.attrs.iter().map(|ident| {
            let ident_string = ident.to_string();
            let attr_camel_name = ident_string
                .strip_prefix("r#")
                .unwrap_or(&ident_string)
                .to_case(Case::UpperCamel);
            let descriptor = Ident::new(
                format!("{camel_name}{attr_camel_name}AttributeDescriptor").as_str(),
                ident.span(),
            );
            let d = quote! { #name::#ident };
            quote! {
                #[doc(hidden)]
                pub struct #descriptor;

                impl ::dioxus_core::view::AttributeDescriptor for #descriptor {
                    const NAME: &'static str = #d.0;
                    const NAMESPACE: ::dioxus_core::TemplateRawAttrNamespace = #d.1;
                    const VOLATILE: bool = #d.2;
                }
            }
        });

        let impls = self.attrs.iter().map(|ident| {
            let ident_string = ident.to_string();
            let attr_camel_name = ident_string
                .strip_prefix("r#")
                .unwrap_or(&ident_string)
                .to_case(Case::UpperCamel);
            let descriptor = Ident::new(
                format!("{camel_name}{attr_camel_name}AttributeDescriptor").as_str(),
                ident.span(),
            );
            quote! {
                fn #ident<__DioxusAttrMarker, __DioxusAttrValue>(
                    self,
                    value: __DioxusAttrValue,
                ) -> <__DioxusAttrValue as ::dioxus_core::view::IntoAttributeBuilderValue<
                    Self,
                    #descriptor,
                    __DioxusAttrMarker,
                >>::Output
                where
                    __DioxusAttrValue: ::dioxus_core::view::IntoAttributeBuilderValue<
                        Self,
                        #descriptor,
                        __DioxusAttrMarker,
                    >,
                {
                    <__DioxusAttrValue as ::dioxus_core::view::IntoAttributeBuilderValue<
                        Self,
                        #descriptor,
                        __DioxusAttrMarker,
                    >>::append_to(value, self)
                }
            }
        });
        let element_impl = self.for_el.then(|| {
            quote! {
                impl<__DioxusAttrs, __DioxusChildren> #extension_name
                    for ::dioxus_core::view::El<#name::Tag, __DioxusAttrs, __DioxusChildren>
                {}
            }
        });
        tokens.append_all(quote! {
            #(#descriptors)*

            pub trait #extension_name: ::dioxus_core::view::AttributeTarget + Sized {
                #(#impls)*
            }

            #element_impl
        });
    }
}
