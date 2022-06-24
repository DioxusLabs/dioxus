use proc_macro2::Ident;
use quote::{format_ident, quote, ToTokens};
use syn::{parse::Parse, ItemFn, Signature, Type, Visibility};

pub struct ComponentFn {
    vis: Visibility,
    name: Ident,
    render_fn: ItemFn,
    props_ty: Type,
}

fn parse_props_type(sig: &Signature) -> Type {
    for input in &sig.inputs {
        match input {
            syn::FnArg::Receiver(_) => {}
            syn::FnArg::Typed(ty) => {
                if let syn::Type::Path(p) = &*ty.ty {
                    for segment in &p.path.segments {
                        if segment.ident == "Scope" {
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                for arg in &args.args {
                                    if let syn::GenericArgument::Type(ty) = arg {
                                        return ty.clone();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    syn::parse_quote!(())
}

impl Parse for ComponentFn {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut render_fn: ItemFn = input.parse()?;

        let name = render_fn.sig.ident;
        render_fn.sig.ident = format_ident!("render");

        let vis = render_fn.vis;
        render_fn.vis = Visibility::Inherited;

        Ok(Self {
            vis,
            props_ty: parse_props_type(&render_fn.sig),
            render_fn,
            name,
        })
    }
}

impl ToTokens for ComponentFn {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = &self.name;
        let vis = &self.vis;
        let props_ty = &self.props_ty;
        let render_fn = &self.render_fn;

        tokens.extend(quote! {
            #vis struct #name;

            impl dioxus::prelude::Component for #name {
                type Props = #props_ty;

                fn renderer(&self) -> fn(dioxus::prelude::Scope<Self::Props>) -> dioxus::prelude::Element {
                    #render_fn
                    render
                }

            }
        });
    }
}
