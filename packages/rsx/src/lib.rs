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
mod component;
mod element;
mod hot_reloading_context;
mod ifmt;
mod node;

use std::{borrow::Borrow, hash::Hash};

// Re-export the namespaces into each other
pub use component::*;
use dioxus_core::{Template, TemplateAttribute, TemplateNode};
pub use element::*;
use hot_reloading_context::{Empty, HotReloadingContext};
pub use ifmt::*;
use internment::Intern;
pub use node::*;

// imports
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Result, Token,
};

// interns a object into a static object, resusing the value if it already exists
fn intern<'a, T: Eq + Hash + Send + Sync + ?Sized + 'static>(
    s: impl Into<Intern<T>>,
) -> &'static T {
    s.into().as_ref()
}

/// Fundametnally, every CallBody is a template
#[derive(Default)]
pub struct CallBody<Ctx: HotReloadingContext = Empty> {
    pub roots: Vec<BodyNode>,

    // set this after
    pub inline_cx: bool,

    phantom: std::marker::PhantomData<Ctx>,
}

impl<Ctx: HotReloadingContext> CallBody<Ctx> {
    /// This will try to create a new template from the current body and the previous body. This will return None if the rsx has some dynamic part that has changed.
    /// This function intentionally leaks memory to create a static template.
    /// Keeping the template static allows us to simplify the core of dioxus and leaking memory in dev mode is less of an issue.
    /// the previous_location is the location of the previous template at the time the template was originally compiled.
    pub fn update_template(
        &self,
        template: Option<&CallBody<Ctx>>,
        location: &'static str,
    ) -> Option<Template> {
        let mut renderer: TemplateRenderer<Ctx> = TemplateRenderer {
            roots: &self.roots,
            phantom: std::marker::PhantomData,
        };
        renderer.update_template(template, location)
    }
}

impl<Ctx: HotReloadingContext> Parse for CallBody<Ctx> {
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
            phantom: std::marker::PhantomData,
        })
    }
}

/// Serialize the same way, regardless of flavor
impl<Ctx: HotReloadingContext> ToTokens for CallBody<Ctx> {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let body: TemplateRenderer<Ctx> = TemplateRenderer {
            roots: &self.roots,
            phantom: std::marker::PhantomData,
        };

        if self.inline_cx {
            out_tokens.append_all(quote! {
                Ok({
                    let __cx = cx;
                    #body
                })
            })
        } else {
            out_tokens.append_all(quote! {
                ::dioxus::core::LazyNodes::new( move | __cx: &::dioxus::core::ScopeState| -> ::dioxus::core::VNode {
                    #body
                })
            })
        }
    }
}

pub struct TemplateRenderer<'a, Ctx: HotReloadingContext = Empty> {
    pub roots: &'a [BodyNode],
    phantom: std::marker::PhantomData<Ctx>,
}

impl<'a, Ctx: HotReloadingContext> TemplateRenderer<'a, Ctx> {
    fn update_template(
        &mut self,
        previous_call: Option<&CallBody<Ctx>>,
        location: &'static str,
    ) -> Option<Template<'static>> {
        let mut context: DynamicContext<Ctx> = DynamicContext::default();

        let roots: Vec<_> = self
            .roots
            .iter()
            .enumerate()
            .map(|(idx, root)| {
                context.current_path.push(idx as u8);
                let out = context.update_node(root);
                context.current_path.pop();
                out
            })
            .collect();

        Some(Template {
            name: location,
            roots: intern(roots.as_slice()),
            node_paths: intern(
                context
                    .node_paths
                    .into_iter()
                    .map(|path| intern(path.as_slice()))
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
            attr_paths: intern(
                context
                    .attr_paths
                    .into_iter()
                    .map(|path| intern(path.as_slice()))
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
        })
    }
}

impl<'a, Ctx: HotReloadingContext> ToTokens for TemplateRenderer<'a, Ctx> {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let mut context: DynamicContext<Ctx> = DynamicContext::default();

        let key = match self.roots.get(0) {
            Some(BodyNode::Element(el)) if self.roots.len() == 1 => el.key.clone(),
            Some(BodyNode::Component(comp)) if self.roots.len() == 1 => comp.key().cloned(),
            _ => None,
        };

        let key_tokens = match key {
            Some(tok) => quote! { Some( __cx.raw_text(#tok) ) },
            None => quote! { None },
        };

        let spndbg = format!("{:?}", self.roots[0].span());
        let root_col = spndbg[9..].split("..").next().unwrap();

        let root_printer = self.roots.iter().enumerate().map(|(idx, root)| {
            context.current_path.push(idx as u8);
            let out = context.render_static_node(root);
            context.current_path.pop();
            out
        });

        // Render and release the mutable borrow on context
        let num_roots = self.roots.len();
        let roots = quote! { #( #root_printer ),* };
        let node_printer = &context.dynamic_nodes;
        let dyn_attr_printer = &context.dynamic_attributes;
        let node_paths = context.node_paths.iter().map(|it| quote!(&[#(#it),*]));
        let attr_paths = context.attr_paths.iter().map(|it| quote!(&[#(#it),*]));

        out_tokens.append_all(quote! {
            static TEMPLATE: ::dioxus::core::Template = ::dioxus::core::Template {
                name: concat!(
                    file!(),
                    ":",
                    line!(),
                    ":",
                    column!(),
                    ":",
                    #root_col
                ),
                roots: &[ #roots ],
                node_paths: &[ #(#node_paths),* ],
                attr_paths: &[ #(#attr_paths),* ],
            };
            ::dioxus::core::VNode {
                parent: None,
                key: #key_tokens,
                template: TEMPLATE,
                root_ids: std::cell::Cell::from_mut( __cx.bump().alloc([::dioxus::core::ElementId(0); #num_roots]) as &mut [::dioxus::core::ElementId]).as_slice_of_cells(),
                dynamic_nodes: __cx.bump().alloc([ #( #node_printer ),* ]),
                dynamic_attrs: __cx.bump().alloc([ #( #dyn_attr_printer ),* ]),
            }
        });
    }
}

// As we create the dynamic nodes, we want to keep track of them in a linear fashion
// We'll use the size of the vecs to determine the index of the dynamic node in the final output
pub struct DynamicContext<'a, Ctx: HotReloadingContext> {
    dynamic_nodes: Vec<&'a BodyNode>,
    dynamic_attributes: Vec<&'a ElementAttrNamed>,
    current_path: Vec<u8>,

    node_paths: Vec<Vec<u8>>,
    attr_paths: Vec<Vec<u8>>,

    phantom: std::marker::PhantomData<Ctx>,
}

impl<'a, Ctx: HotReloadingContext> Default for DynamicContext<'a, Ctx> {
    fn default() -> Self {
        Self {
            dynamic_nodes: Vec::new(),
            dynamic_attributes: Vec::new(),
            current_path: Vec::new(),
            node_paths: Vec::new(),
            attr_paths: Vec::new(),
            phantom: std::marker::PhantomData,
        }
    }
}

impl<'a, Ctx: HotReloadingContext> DynamicContext<'a, Ctx> {
    fn update_node(&mut self, root: &'a BodyNode) -> TemplateNode<'static> {
        match root {
            BodyNode::Element(el) => {
                // dynamic attributes
                // [0]
                // [0, 1]
                // [0, 1]
                // [0, 1]
                // [0, 1, 2]
                // [0, 2]
                // [0, 2, 1]

                let element_name_rust = el.name.to_string();

                let static_attrs: Vec<TemplateAttribute<'static>> = el
                    .attributes
                    .iter()
                    .map(|attr| match &attr.attr {
                        ElementAttr::AttrText { name, value } if value.is_static() => {
                            let value = value.source.as_ref().unwrap();
                            let attribute_name_rust = name.to_string();
                            let (name, namespace) =
                                Ctx::map_attribute(&element_name_rust, &attribute_name_rust)
                                    .unwrap_or((intern(attribute_name_rust.as_str()), None));
                            TemplateAttribute::Static {
                                name,
                                namespace,
                                value: intern(value.value().as_str()),
                                // name: dioxus_elements::#el_name::#name.0,
                                // namespace: dioxus_elements::#el_name::#name.1,
                                // value: #value,

                                // todo: we don't diff these so we never apply the volatile flag
                                // volatile: dioxus_elements::#el_name::#name.2,
                            }
                        }

                        ElementAttr::CustomAttrText { name, value } if value.is_static() => {
                            let value = value.source.as_ref().unwrap();
                            TemplateAttribute::Static {
                                name: intern(name.value().as_str()),
                                namespace: None,
                                value: intern(value.value().as_str()),
                                // todo: we don't diff these so we never apply the volatile flag
                                // volatile: dioxus_elements::#el_name::#name.2,
                            }
                        }

                        ElementAttr::AttrExpression { .. }
                        | ElementAttr::AttrText { .. }
                        | ElementAttr::CustomAttrText { .. }
                        | ElementAttr::CustomAttrExpression { .. }
                        | ElementAttr::EventTokens { .. } => {
                            let ct = self.dynamic_attributes.len();
                            self.dynamic_attributes.push(attr);
                            self.attr_paths.push(self.current_path.clone());
                            TemplateAttribute::Dynamic { id: ct }
                        }
                    })
                    .collect();

                let children: Vec<_> = el
                    .children
                    .iter()
                    .enumerate()
                    .map(|(idx, root)| {
                        self.current_path.push(idx as u8);
                        let out = self.update_node(root);
                        self.current_path.pop();
                        out
                    })
                    .collect();

                let (tag, namespace) = Ctx::map_element(&element_name_rust)
                    .unwrap_or((intern(element_name_rust.as_str()), None));
                TemplateNode::Element {
                    tag,
                    namespace,
                    attrs: intern(static_attrs.into_boxed_slice()),
                    children: intern(children.as_slice()),
                }
            }

            BodyNode::Text(text) if text.is_static() => {
                let text = text.source.as_ref().unwrap();
                TemplateNode::Text {
                    text: intern(text.value().as_str()),
                }
            }

            BodyNode::RawExpr(_)
            | BodyNode::Text(_)
            | BodyNode::ForLoop(_)
            | BodyNode::IfChain(_)
            | BodyNode::Component(_) => {
                let ct = self.dynamic_nodes.len();
                self.dynamic_nodes.push(root);
                self.node_paths.push(self.current_path.clone());

                match root {
                    BodyNode::Text(_) => TemplateNode::DynamicText { id: ct },
                    _ => TemplateNode::Dynamic { id: ct },
                }
            }
        }
    }

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

                let static_attrs = el.attributes.iter().map(|attr| match &attr.attr {
                    ElementAttr::AttrText { name, value } if value.is_static() => {
                        let value = value.source.as_ref().unwrap();
                        quote! {
                            ::dioxus::core::TemplateAttribute::Static {
                                name: dioxus_elements::#el_name::#name.0,
                                namespace: dioxus_elements::#el_name::#name.1,
                                value: #value,

                                // todo: we don't diff these so we never apply the volatile flag
                                // volatile: dioxus_elements::#el_name::#name.2,
                            }
                        }
                    }

                    ElementAttr::CustomAttrText { name, value } if value.is_static() => {
                        let value = value.source.as_ref().unwrap();
                        quote! {
                            ::dioxus::core::TemplateAttribute::Static {
                                name: #name,
                                namespace: None,
                                value: #value,

                                // todo: we don't diff these so we never apply the volatile flag
                                // volatile: dioxus_elements::#el_name::#name.2,
                            }
                        }
                    }

                    ElementAttr::AttrExpression { .. }
                    | ElementAttr::AttrText { .. }
                    | ElementAttr::CustomAttrText { .. }
                    | ElementAttr::CustomAttrExpression { .. }
                    | ElementAttr::EventTokens { .. } => {
                        let ct = self.dynamic_attributes.len();
                        self.dynamic_attributes.push(attr);
                        self.attr_paths.push(self.current_path.clone());
                        quote! { ::dioxus::core::TemplateAttribute::Dynamic { id: #ct } }
                    }
                });

                let attrs = quote! { #(#static_attrs),*};

                let children = el.children.iter().enumerate().map(|(idx, root)| {
                    self.current_path.push(idx as u8);
                    let out = self.render_static_node(root);
                    self.current_path.pop();
                    out
                });

                let _opt = el.children.len() == 1;
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
                quote! { ::dioxus::core::TemplateNode::Text{ text: #text } }
            }

            BodyNode::RawExpr(_)
            | BodyNode::Text(_)
            | BodyNode::ForLoop(_)
            | BodyNode::IfChain(_)
            | BodyNode::Component(_) => {
                let ct = self.dynamic_nodes.len();
                self.dynamic_nodes.push(root);
                self.node_paths.push(self.current_path.clone());

                match root {
                    BodyNode::Text(_) => {
                        quote! { ::dioxus::core::TemplateNode::DynamicText { id: #ct } }
                    }
                    _ => quote! { ::dioxus::core::TemplateNode::Dynamic { id: #ct } },
                }
            }
        }
    }
}

#[test]
fn template() {
    let input = quote! {
        svg {
            width: 100,
            height: "100px",
            "width2": 100,
            "height2": "100px",
            p {
                "hello world"
            }
            (0..10).map(|i| rsx!{"{i}"})
        }
    };

    struct Mock;

    impl HotReloadingContext for Mock {
        fn map_attribute(
            element_name_rust: &str,
            attribute_name_rust: &str,
        ) -> Option<(&'static str, Option<&'static str>)> {
            match element_name_rust {
                "svg" => match attribute_name_rust {
                    "width" => Some(("width", Some("style"))),
                    "height" => Some(("height", Some("style"))),
                    _ => None,
                },
                _ => None,
            }
        }

        fn map_element(element_name_rust: &str) -> Option<(&'static str, Option<&'static str>)> {
            match element_name_rust {
                "svg" => Some(("svg", Some("svg"))),
                _ => None,
            }
        }
    }

    let call_body: CallBody<Mock> = syn::parse2(input).unwrap();

    let template = call_body.update_template(None, "testing").unwrap();

    dbg!(template);

    assert_eq!(
        template,
        Template {
            name: "testing",
            roots: &[TemplateNode::Element {
                tag: "svg",
                namespace: Some("svg"),
                attrs: &[
                    TemplateAttribute::Dynamic { id: 0 },
                    TemplateAttribute::Static {
                        name: "height",
                        namespace: Some("style"),
                        value: "100px",
                    },
                    TemplateAttribute::Dynamic { id: 1 },
                    TemplateAttribute::Static {
                        name: "height2",
                        namespace: None,
                        value: "100px",
                    },
                ],
                children: &[
                    TemplateNode::Element {
                        tag: "p",
                        namespace: None,
                        attrs: &[],
                        children: &[TemplateNode::Text {
                            text: "hello world",
                        }],
                    },
                    TemplateNode::Dynamic { id: 0 }
                ],
            }],
            node_paths: &[&[0, 1,],],
            attr_paths: &[&[0,], &[0,],],
        },
    )
}
