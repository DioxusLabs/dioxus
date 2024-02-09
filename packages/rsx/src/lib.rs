#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

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
mod attribute;
mod component;
mod element;
#[cfg(feature = "hot_reload")]
pub mod hot_reload;
mod ifmt;
mod node;

use std::{fmt::Debug, hash::Hash};

// Re-export the namespaces into each other
pub use attribute::*;
pub use component::*;
#[cfg(feature = "hot_reload")]
use dioxus_core::{Template, TemplateAttribute, TemplateNode};
pub use element::*;
#[cfg(feature = "hot_reload")]
pub use hot_reload::HotReloadingContext;
pub use ifmt::*;
#[cfg(feature = "hot_reload")]
use internment::Intern;
pub use node::*;

// imports
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Result, Token,
};

#[cfg(feature = "hot_reload")]
// interns a object into a static object, resusing the value if it already exists
fn intern<T: Eq + Hash + Send + Sync + ?Sized + 'static>(s: impl Into<Intern<T>>) -> &'static T {
    s.into().as_ref()
}

/// Fundametnally, every CallBody is a template
#[derive(Default, Debug)]
pub struct CallBody {
    pub roots: Vec<BodyNode>,
}

impl CallBody {
    #[cfg(feature = "hot_reload")]
    /// This will try to create a new template from the current body and the previous body. This will return None if the rsx has some dynamic part that has changed.
    /// This function intentionally leaks memory to create a static template.
    /// Keeping the template static allows us to simplify the core of dioxus and leaking memory in dev mode is less of an issue.
    /// the previous_location is the location of the previous template at the time the template was originally compiled.
    pub fn update_template<Ctx: HotReloadingContext>(
        &self,
        template: Option<CallBody>,
        location: &'static str,
    ) -> Option<Template> {
        let mut renderer: TemplateRenderer = TemplateRenderer {
            roots: &self.roots,
            location: None,
        };
        renderer.update_template::<Ctx>(template, location)
    }

    /// Render the template with a manually set file location. This should be used when multiple rsx! calls are used in the same macro
    pub fn render_with_location(&self, location: String) -> TokenStream2 {
        let body = TemplateRenderer {
            roots: &self.roots,
            location: Some(location),
        };

        quote! {
            Some({
                #body
            })
        }
    }
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

#[derive(Default, Debug)]
pub struct RenderCallBody(pub CallBody);

impl ToTokens for RenderCallBody {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let body: TemplateRenderer = TemplateRenderer {
            roots: &self.0.roots,
            location: None,
        };

        out_tokens.append_all(quote! {
            Some({
                #body
            })
        })
    }
}

pub struct TemplateRenderer<'a> {
    pub roots: &'a [BodyNode],
    pub location: Option<String>,
}

impl<'a> TemplateRenderer<'a> {
    #[cfg(feature = "hot_reload")]
    fn update_template<Ctx: HotReloadingContext>(
        &mut self,
        previous_call: Option<CallBody>,
        location: &'static str,
    ) -> Option<Template> {
        let mut mapping = previous_call.map(|call| DynamicMapping::from(call.roots));

        let mut context = DynamicContext::default();

        let mut roots = Vec::new();
        for (idx, root) in self.roots.iter().enumerate() {
            context.current_path.push(idx as u8);
            roots.push(context.update_node::<Ctx>(root, &mut mapping)?);
            context.current_path.pop();
        }

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

impl<'a> ToTokens for TemplateRenderer<'a> {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let mut context = DynamicContext::default();

        let key = match self.roots.first() {
            Some(BodyNode::Element(el)) if self.roots.len() == 1 => el.key.clone(),
            Some(BodyNode::Component(comp)) if self.roots.len() == 1 => comp.key().cloned(),
            _ => None,
        };

        let key_tokens = match key {
            Some(tok) => quote! { Some( #tok.to_string() ) },
            None => quote! { None },
        };

        let root_col = match self.roots.first() {
            Some(first_root) => {
                let first_root_span = format!("{:?}", first_root.span());
                first_root_span
                    .rsplit_once("..")
                    .and_then(|(_, after)| after.split_once(')').map(|(before, _)| before))
                    .unwrap_or_default()
                    .to_string()
            }
            _ => "0".to_string(),
        };
        let root_printer = self.roots.iter().enumerate().map(|(idx, root)| {
            context.current_path.push(idx as u8);
            let out = context.render_static_node(root);
            context.current_path.pop();
            out
        });

        let name = match self.location {
            Some(ref loc) => quote! { #loc },
            None => quote! {
                concat!(
                    file!(),
                    ":",
                    line!(),
                    ":",
                    column!(),
                    ":",
                    #root_col
                )
            },
        };

        // Render and release the mutable borrow on context
        let roots = quote! { #( #root_printer ),* };
        let node_printer = &context.dynamic_nodes;
        let dyn_attr_printer = context
            .dynamic_attributes
            .iter()
            .map(|attrs| AttributeType::merge_quote(attrs));
        let node_paths = context.node_paths.iter().map(|it| quote!(&[#(#it),*]));
        let attr_paths = context.attr_paths.iter().map(|it| quote!(&[#(#it),*]));

        out_tokens.append_all(quote! {
            static TEMPLATE: dioxus_core::Template = dioxus_core::Template {
                name: #name,
                roots: &[ #roots ],
                node_paths: &[ #(#node_paths),* ],
                attr_paths: &[ #(#attr_paths),* ],
            };

            dioxus_core::VNode::new(
                #key_tokens,
                TEMPLATE,
                Box::new([ #( #node_printer),* ]),
                Box::new([ #(#dyn_attr_printer),* ]),
            )
        });
    }
}

#[cfg(feature = "hot_reload")]
#[derive(Default, Debug)]
struct DynamicMapping {
    attribute_to_idx: std::collections::HashMap<AttributeType, Vec<usize>>,
    last_attribute_idx: usize,
    node_to_idx: std::collections::HashMap<BodyNode, Vec<usize>>,
    last_element_idx: usize,
}

#[cfg(feature = "hot_reload")]
impl DynamicMapping {
    fn from(nodes: Vec<BodyNode>) -> Self {
        let mut new = Self::default();
        for node in nodes {
            new.add_node(node);
        }
        new
    }

    fn get_attribute_idx(&mut self, attr: &AttributeType) -> Option<usize> {
        self.attribute_to_idx
            .get_mut(attr)
            .and_then(|idxs| idxs.pop())
    }

    fn get_node_idx(&mut self, node: &BodyNode) -> Option<usize> {
        self.node_to_idx.get_mut(node).and_then(|idxs| idxs.pop())
    }

    fn insert_attribute(&mut self, attr: AttributeType) -> usize {
        let idx = self.last_attribute_idx;
        self.last_attribute_idx += 1;

        self.attribute_to_idx.entry(attr).or_default().push(idx);

        idx
    }

    fn insert_node(&mut self, node: BodyNode) -> usize {
        let idx = self.last_element_idx;
        self.last_element_idx += 1;

        self.node_to_idx.entry(node).or_default().push(idx);

        idx
    }

    fn add_node(&mut self, node: BodyNode) {
        match node {
            BodyNode::Element(el) => {
                for attr in el.merged_attributes {
                    match &attr {
                        AttributeType::Named(ElementAttrNamed {
                            attr:
                                ElementAttr {
                                    value: ElementAttrValue::AttrLiteral(input),
                                    ..
                                },
                            ..
                        }) if input.is_static() => {}
                        _ => {
                            self.insert_attribute(attr);
                        }
                    }
                }

                for child in el.children {
                    self.add_node(child);
                }
            }

            BodyNode::Text(text) if text.is_static() => {}

            BodyNode::RawExpr(_)
            | BodyNode::Text(_)
            | BodyNode::ForLoop(_)
            | BodyNode::IfChain(_)
            | BodyNode::Component(_) => {
                self.insert_node(node);
            }
        }
    }
}

// As we create the dynamic nodes, we want to keep track of them in a linear fashion
// We'll use the size of the vecs to determine the index of the dynamic node in the final output
#[derive(Default, Debug)]
pub struct DynamicContext<'a> {
    dynamic_nodes: Vec<&'a BodyNode>,
    dynamic_attributes: Vec<Vec<&'a AttributeType>>,
    current_path: Vec<u8>,

    node_paths: Vec<Vec<u8>>,
    attr_paths: Vec<Vec<u8>>,
}

impl<'a> DynamicContext<'a> {
    #[cfg(feature = "hot_reload")]
    fn update_node<Ctx: HotReloadingContext>(
        &mut self,
        root: &'a BodyNode,
        mapping: &mut Option<DynamicMapping>,
    ) -> Option<TemplateNode> {
        match root {
            BodyNode::Element(el) => {
                let element_name_rust = el.name.to_string();

                let mut static_attrs = Vec::new();
                for attr in &el.merged_attributes {
                    match &attr {
                        AttributeType::Named(ElementAttrNamed {
                            attr:
                                ElementAttr {
                                    value: ElementAttrValue::AttrLiteral(value),
                                    name,
                                },
                            ..
                        }) if value.is_static() => {
                            let value = value.source.as_ref().unwrap();
                            let attribute_name_rust = name.to_string();
                            let (name, namespace) =
                                Ctx::map_attribute(&element_name_rust, &attribute_name_rust)
                                    .unwrap_or((intern(attribute_name_rust.as_str()), None));
                            static_attrs.push(TemplateAttribute::Static {
                                name,
                                namespace,
                                value: intern(value.value().as_str()),
                            })
                        }

                        _ => {
                            let idx = match mapping {
                                Some(mapping) => mapping.get_attribute_idx(attr)?,
                                None => self.dynamic_attributes.len(),
                            };
                            self.dynamic_attributes.push(vec![attr]);

                            if self.attr_paths.len() <= idx {
                                self.attr_paths.resize_with(idx + 1, Vec::new);
                            }
                            self.attr_paths[idx] = self.current_path.clone();
                            static_attrs.push(TemplateAttribute::Dynamic { id: idx })
                        }
                    }
                }

                let mut children = Vec::new();
                for (idx, root) in el.children.iter().enumerate() {
                    self.current_path.push(idx as u8);
                    children.push(self.update_node::<Ctx>(root, mapping)?);
                    self.current_path.pop();
                }

                let (tag, namespace) = Ctx::map_element(&element_name_rust)
                    .unwrap_or((intern(element_name_rust.as_str()), None));
                Some(TemplateNode::Element {
                    tag,
                    namespace,
                    attrs: intern(static_attrs.into_boxed_slice()),
                    children: intern(children.as_slice()),
                })
            }

            BodyNode::Text(text) if text.is_static() => {
                let text = text.source.as_ref().unwrap();
                Some(TemplateNode::Text {
                    text: intern(text.value().as_str()),
                })
            }

            BodyNode::RawExpr(_)
            | BodyNode::Text(_)
            | BodyNode::ForLoop(_)
            | BodyNode::IfChain(_)
            | BodyNode::Component(_) => {
                let idx = match mapping {
                    Some(mapping) => mapping.get_node_idx(root)?,
                    None => self.dynamic_nodes.len(),
                };
                self.dynamic_nodes.push(root);

                if self.node_paths.len() <= idx {
                    self.node_paths.resize_with(idx + 1, Vec::new);
                }
                self.node_paths[idx] = self.current_path.clone();

                Some(match root {
                    BodyNode::Text(_) => TemplateNode::DynamicText { id: idx },
                    _ => TemplateNode::Dynamic { id: idx },
                })
            }
        }
    }

    fn render_static_node(&mut self, root: &'a BodyNode) -> TokenStream2 {
        match root {
            BodyNode::Element(el) => {
                let el_name = &el.name;
                let ns = |name| match el_name {
                    ElementName::Ident(i) => quote! { dioxus_elements::#i::#name },
                    ElementName::Custom(_) => quote! { None },
                };
                let static_attrs = el.merged_attributes.iter().map(|attr| match attr {
                    AttributeType::Named(ElementAttrNamed {
                        attr:
                            ElementAttr {
                                value: ElementAttrValue::AttrLiteral(value),
                                name,
                            },
                        ..
                    }) if value.is_static() => {
                        let value = value.to_static().unwrap();
                        let ns = {
                            match name {
                                ElementAttrName::BuiltIn(name) => ns(quote!(#name.1)),
                                ElementAttrName::Custom(_) => quote!(None),
                            }
                        };
                        let name = match (el_name, name) {
                            (ElementName::Ident(_), ElementAttrName::BuiltIn(_)) => {
                                quote! { #el_name::#name.0 }
                            }
                            _ => {
                                let as_string = name.to_string();
                                quote! { #as_string }
                            }
                        };
                        quote! {
                            dioxus_core::TemplateAttribute::Static {
                                name: #name,
                                namespace: #ns,
                                value: #value,

                                // todo: we don't diff these so we never apply the volatile flag
                                // volatile: dioxus_elements::#el_name::#name.2,
                            },
                        }
                    }

                    _ => {
                        // If this attribute is dynamic, but it already exists in the template, we can reuse the index
                        if let Some(attribute_index) = self
                            .attr_paths
                            .iter()
                            .position(|path| path == &self.current_path)
                        {
                            self.dynamic_attributes[attribute_index].push(attr);
                            quote! {}
                        } else {
                            let ct = self.dynamic_attributes.len();
                            self.dynamic_attributes.push(vec![attr]);
                            self.attr_paths.push(self.current_path.clone());
                            quote! { dioxus_core::TemplateAttribute::Dynamic { id: #ct }, }
                        }
                    }
                });

                let attrs = quote! { #(#static_attrs)* };

                let children = el.children.iter().enumerate().map(|(idx, root)| {
                    self.current_path.push(idx as u8);
                    let out = self.render_static_node(root);
                    self.current_path.pop();
                    out
                });

                let _opt = el.children.len() == 1;
                let children = quote! { #(#children),* };

                let ns = ns(quote!(NAME_SPACE));
                let el_name = el_name.tag_name();

                quote! {
                    dioxus_core::TemplateNode::Element {
                        tag: #el_name,
                        namespace: #ns,
                        attrs: &[ #attrs ],
                        children: &[ #children ],
                    }
                }
            }

            BodyNode::Text(text) if text.is_static() => {
                let text = text.to_static().unwrap();
                quote! { dioxus_core::TemplateNode::Text{ text: #text } }
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
                        quote! { dioxus_core::TemplateNode::DynamicText { id: #ct } }
                    }
                    _ => quote! { dioxus_core::TemplateNode::Dynamic { id: #ct } },
                }
            }
        }
    }
}

#[cfg(feature = "hot_reload")]
#[test]
fn create_template() {
    let input = quote! {
        svg {
            width: 100,
            height: "100px",
            "width2": 100,
            "height2": "100px",
            p {
                "hello world"
            }
            {(0..10).map(|i| rsx!{"{i}"})}
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

    let call_body: CallBody = syn::parse2(input).unwrap();

    let template = call_body.update_template::<Mock>(None, "testing").unwrap();

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

#[cfg(feature = "hot_reload")]
#[test]
fn diff_template() {
    #[allow(unused, non_snake_case)]
    fn Comp() -> dioxus_core::Element {
        None
    }

    let input = quote! {
        svg {
            width: 100,
            height: "100px",
            "width2": 100,
            "height2": "100px",
            p {
                "hello world"
            }
            {(0..10).map(|i| rsx!{"{i}"})},
            {(0..10).map(|i| rsx!{"{i}"})},
            {(0..11).map(|i| rsx!{"{i}"})},
            Comp{}
        }
    };

    #[derive(Debug)]
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

    let call_body1: CallBody = syn::parse2(input).unwrap();

    let template = call_body1.update_template::<Mock>(None, "testing").unwrap();
    dbg!(template);

    // scrambling the attributes should not cause a full rebuild
    let input = quote! {
        div {
            "width2": 100,
            height: "100px",
            "height2": "100px",
            width: 100,
            Comp{}
            {(0..11).map(|i| rsx!{"{i}"})},
            {(0..10).map(|i| rsx!{"{i}"})},
            {(0..10).map(|i| rsx!{"{i}"})},
            p {
                "hello world"
            }
        }
    };

    let call_body2: CallBody = syn::parse2(input).unwrap();

    let template = call_body2
        .update_template::<Mock>(Some(call_body1), "testing")
        .unwrap();
    dbg!(template);

    assert_eq!(
        template,
        Template {
            name: "testing",
            roots: &[TemplateNode::Element {
                tag: "div",
                namespace: None,
                attrs: &[
                    TemplateAttribute::Dynamic { id: 1 },
                    TemplateAttribute::Static {
                        name: "height",
                        namespace: None,
                        value: "100px",
                    },
                    TemplateAttribute::Static {
                        name: "height2",
                        namespace: None,
                        value: "100px",
                    },
                    TemplateAttribute::Dynamic { id: 0 },
                ],
                children: &[
                    TemplateNode::Dynamic { id: 3 },
                    TemplateNode::Dynamic { id: 2 },
                    TemplateNode::Dynamic { id: 1 },
                    TemplateNode::Dynamic { id: 0 },
                    TemplateNode::Element {
                        tag: "p",
                        namespace: None,
                        attrs: &[],
                        children: &[TemplateNode::Text {
                            text: "hello world",
                        }],
                    },
                ],
            }],
            node_paths: &[&[0, 3], &[0, 2], &[0, 1], &[0, 0]],
            attr_paths: &[&[0], &[0]]
        },
    )
}
