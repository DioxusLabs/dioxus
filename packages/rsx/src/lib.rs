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

        Ok(Self { roots })
    }
}

/// Serialize the same way, regardless of flavor
impl ToTokens for CallBody {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        // As we print out the dynamic nodes, we want to keep track of them in a linear fashion
        // We'll use the size of the vecs to determine the index of the dynamic node in the final
        struct DynamicContext<'a> {
            dynamic_nodes: Vec<&'a BodyNode>,
            dynamic_attributes: Vec<&'a ElementAttrNamed>,
            dynamic_listeners: Vec<&'a ElementAttrNamed>,
        }

        let mut context = DynamicContext {
            dynamic_nodes: vec![],
            dynamic_attributes: vec![],
            dynamic_listeners: vec![],
        };

        fn render_static_node<'a>(root: &'a BodyNode, cx: &mut DynamicContext<'a>) -> TokenStream2 {
            match root {
                BodyNode::Element(el) => {
                    let el_name = &el.name;

                    let children = {
                        let children = el.children.iter().map(|root| render_static_node(root, cx));
                        quote! { #(#children),* }
                    };

                    let attrs = el.attributes.iter().filter_map(|attr| {
                        //
                        match &attr.attr {
                            ElementAttr::AttrText { name, value } if value.is_static() => {
                                let value = value.source.as_ref().unwrap();
                                Some(quote! {
                                    ::dioxus::core::TemplateAttribute {
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
                                    ::dioxus::core::TemplateAttribute {
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
                                let ct = cx.dynamic_attributes.len();
                                cx.dynamic_attributes.push(attr);
                                // quote! {}
                                None
                                // quote! { ::dioxus::core::TemplateAttribute::Dynamic(#ct) }
                            }

                            ElementAttr::EventTokens { .. } => {
                                let ct = cx.dynamic_listeners.len();
                                cx.dynamic_listeners.push(attr);
                                // quote! {}
                                None
                            }
                        }
                    });

                    quote! {
                        ::dioxus::core::TemplateNode::Element {
                            tag: dioxus_elements::#el_name::TAG_NAME,
                            namespace: dioxus_elements::#el_name::NAME_SPACE,
                            attrs: &[ #(#attrs),* ],
                            children: &[ #children ],
                        }
                    }
                }

                BodyNode::Text(text) if text.is_static() => {
                    let text = text.source.as_ref().unwrap();
                    quote! { ::dioxus::core::TemplateNode::Text(#text) }
                }

                BodyNode::RawExpr(_) | BodyNode::Component(_) | BodyNode::Text(_) => {
                    let ct = cx.dynamic_nodes.len();
                    cx.dynamic_nodes.push(root);
                    quote! { ::dioxus::core::TemplateNode::Dynamic(#ct) }
                }
            }
        }

        let root_printer = self
            .roots
            .iter()
            .map(|root| render_static_node(root, &mut context));

        // Render and release the mutable borrow on context
        let roots = quote! { #( #root_printer ),* };

        let node_printer = &context.dynamic_nodes;
        let attr_printer = context.dynamic_attributes.iter();
        let listener_printer = context.dynamic_listeners.iter();

        out_tokens.append_all(quote! {
            // LazyNodes::new(move | __cx: ::dioxus::core::NodeFactory| -> ::dioxus::core::VNode {
            //     __cx.template_ref(
            //         ::dioxus::core::Template {
            //             id: ::dioxus::core::get_line_num!(),
            //             roots: &[ #roots ]
            //         },
            //         __cx.bump().alloc([
            //            #( #node_printer ),*
            //         ]),
            //         __cx.bump().alloc([
            //            #( #attr_printer ),*
            //         ]),
            //         __cx.bump().alloc([
            //            #( #listener_printer ),*
            //         ]),
            //         None
            //     )
            // })


            ::dioxus::core::LazyNodes::new( move | __cx: ::dioxus::core::NodeFactory| -> ::dioxus::core::VNode {
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
                    dynamic_attrs: __cx.bump().alloc([]),
                }
            })
        })
    }
}

impl CallBody {
    pub fn to_tokens_without_template(&self, out_tokens: &mut TokenStream2) {

        // let children = &self.roots;
        // let inner = if children.len() == 1 {
        //     let inner = &self.roots[0];
        //     quote! { #inner }
        // } else {
        //     quote! { __cx.fragment_root([ #(#children),* ]) }
        // };

        // out_tokens.append_all(quote! {
        //     LazyNodes::new(move |__cx: NodeFactory| -> VNode {
        //         use dioxus_elements::{GlobalAttributes, SvgAttributes};
        //         #inner
        //     })
        // })
    }

    pub fn to_tokens_without_lazynodes(&self, out_tokens: &mut TokenStream2) {
        out_tokens.append_all(quote! {
            static TEMPLATE: ::dioxus::core::Template = ::dioxus::core::Template {
                id: ::dioxus::core::get_line_num!(),
                roots: &[]
            };

            LazyNodes::new( move | __cx: ::dioxus::core::NodeFactory| -> ::dioxus::core::VNode {
                ::dioxus::core::VNode {
                    node_id: Default::default(),
                    parent: None,
                    template: &TEMPLATE,
                    root_ids: __cx.bump().alloc([]),
                    dynamic_nodes: __cx.bump().alloc([]),
                    dynamic_attrs: __cx.bump().alloc([]),
                }
            })
        })
    }
}
