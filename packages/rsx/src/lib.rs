//! Parse the root tokens in the rsx!{} macro
//! =========================================
//!
//! This parsing path emerges directly from the macro call, with `RsxRender` being the primary entrance into parsing.
//! This feature must support:
//! - [x] Optionally rendering if the `in XYZ` pattern is present
//! - [x] Fragments as top-level element (through ambiguous)
//! - [x] Components as top-level element (through ambiguous)
//! - [x] Tags as top-level elements (through ambiguous)
//! - [x] Good errors if parsing fails
//!
//! Any errors in using rsx! will likely occur when people start using it, so the first errors must be really helpful.

#[macro_use]
mod errors;
// #[cfg(any(feature = "hot-reload", debug_assertions))]
// mod attributes;
mod component;
mod element;
mod ifmt;
mod node;
mod template;

use std::collections::HashMap;

// Re-export the namespaces into each other
pub use component::*;
pub use element::*;
pub use ifmt::*;
pub use node::*;

// imports
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Result, Token,
};

/// Fundametnally, every CallBody is a template
#[derive(Default)]
pub struct CallBody {
    pub roots: Vec<BodyNode>,

    // set this after
    pub inline_cx: bool,
}

impl Parse for CallBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut roots = Vec::new();

        while !input.is_empty() {
            let node = input.parse::<BodyNode>()?;

            if input.peek(Token![,]) {
                let _ = input.parse::<Token![,]>();
            }

            roots.push(node);
        }

        Ok(Self {
            roots,
            inline_cx: false,
        })
    }
}

/// Serialize the same way, regardless of flavor
impl ToTokens for CallBody {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let body = TemplateRenderer { roots: &self.roots };

        if self.inline_cx {
            out_tokens.append_all(quote! {
                Some({
                    let __cx = cx;
                    #body
                })
            })
        } else {
            out_tokens.append_all(quote! {
                ::dioxus::core::LazyNodes::new( move | __cx: ::dioxus::core::NodeFactory| -> ::dioxus::core::VNode {
                    #body
                })
            })
        }
    }
}

pub struct TemplateRenderer<'a> {
    pub roots: &'a [BodyNode],
}

impl<'a> ToTokens for TemplateRenderer<'a> {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let mut context = DynamicContext {
            dynamic_nodes: vec![],
            dynamic_attributes: vec![],
            current_path: vec![],
        };

        let root_printer = self.roots.iter().enumerate().map(|(idx, root)| {
            context.current_path.push(idx as u8);
            let out = context.render_static_node(root);
            context.current_path.pop();
            out
        });

        // Render and release the mutable borrow on context
        let roots = quote! { #( #root_printer ),* };
        let node_printer = &context.dynamic_nodes;
        let dyn_attr_printer = &context.dynamic_attributes;

        out_tokens.append_all(quote! {
            static TEMPLATE: ::dioxus::core::Template = ::dioxus::core::Template {
                id: ::dioxus::core::get_line_num!(),
                roots: &[ #roots ]
            };
            ::dioxus::core::VNode {
                node_id: Default::default(),
                parent: None,
                template: TEMPLATE,
                root_ids: __cx.bump().alloc([]),
                dynamic_nodes: __cx.bump().alloc([ #( #node_printer ),* ]),
                dynamic_attrs: __cx.bump().alloc([ #( #dyn_attr_printer ),* ]),
            }
        });
    }
}
// As we print out the dynamic nodes, we want to keep track of them in a linear fashion
// We'll use the size of the vecs to determine the index of the dynamic node in the final
pub struct DynamicContext<'a> {
    dynamic_nodes: Vec<&'a BodyNode>,
    dynamic_attributes: Vec<&'a ElementAttrNamed>,
    current_path: Vec<u8>,
}

impl<'a> DynamicContext<'a> {
    fn render_static_node(&mut self, root: &'a BodyNode) -> TokenStream2 {
        match root {
            BodyNode::Element(el) => {
                let el_name = &el.name;

                // dynamic attributes
                // [0]
                // [0, 1]
                // [0, 1]
                // [0, 1]
                // [0, 1, 2]
                // [0, 2]
                // [0, 2, 1]

                let static_attrs = el.attributes.iter().filter_map(|attr| match &attr.attr {
                    ElementAttr::AttrText { name, value } if value.is_static() => {
                        let value = value.source.as_ref().unwrap();
                        Some(quote! {
                            ::dioxus::core::TemplateAttribute::Static {
                                name: dioxus_elements::#el_name::#name.0,
                                namespace: dioxus_elements::#el_name::#name.1,
                                volatile: dioxus_elements::#el_name::#name.2,
                                value: #value,
                            }
                        })
                    }

                    ElementAttr::CustomAttrText { name, value } if value.is_static() => {
                        let value = value.source.as_ref().unwrap();
                        Some(quote! {
                            ::dioxus::core::TemplateAttribute::Static {
                                name: dioxus_elements::#el_name::#name.0,
                                namespace: dioxus_elements::#el_name::#name.1,
                                volatile: dioxus_elements::#el_name::#name.2,
                                value: #value,
                            }
                        })
                    }

                    ElementAttr::AttrExpression { .. }
                    | ElementAttr::AttrText { .. }
                    | ElementAttr::CustomAttrText { .. }
                    | ElementAttr::CustomAttrExpression { .. } => {
                        let ct = self.dynamic_attributes.len();
                        self.dynamic_attributes.push(attr);
                        Some(quote! { ::dioxus::core::TemplateAttribute::Dynamic {
                            name: "asd",
                            index: #ct
                        } })
                    }

                    ElementAttr::EventTokens { .. } => {
                        let ct = self.dynamic_attributes.len();
                        self.dynamic_attributes.push(attr);
                        Some(quote! { ::dioxus::core::TemplateAttribute::Dynamic {
                            name: "asd",
                            index: #ct
                        } })
                    }
                });

                let attrs = quote! { #(#static_attrs),*};

                let children = el.children.iter().enumerate().map(|(idx, root)| {
                    self.current_path.push(idx as u8);
                    let out = self.render_static_node(root);
                    self.current_path.pop();
                    out
                });

                let children = quote! { #(#children),* };

                quote! {
                    ::dioxus::core::TemplateNode::Element {
                        tag: dioxus_elements::#el_name::TAG_NAME,
                        namespace: dioxus_elements::#el_name::NAME_SPACE,
                        attrs: &[ #attrs ],
                        children: &[ #children ],
                    }
                }
            }

            BodyNode::Text(text) if text.is_static() => {
                let text = text.source.as_ref().unwrap();
                quote! { ::dioxus::core::TemplateNode::Text(#text) }
            }

            BodyNode::RawExpr(_) | BodyNode::Text(_) | BodyNode::Component(_) => {
                let ct = self.dynamic_nodes.len();
                self.dynamic_nodes.push(root);
                quote! { ::dioxus::core::TemplateNode::Dynamic(#ct) }
            }
        }
    }
}
