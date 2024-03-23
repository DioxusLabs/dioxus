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
mod ifmt;
mod node;

pub(crate) mod context;
pub(crate) mod mapping;
pub(crate) mod renderer;

use std::{fmt::Debug, hash::Hash};

mod sub_templates;

// Re-export the namespaces into each other
pub use attribute::*;
pub use component::*;
use context::DynamicContext;
pub use element::*;
pub use ifmt::*;
pub use node::*;

#[cfg(feature = "hot_reload")]
pub mod hot_reload;

#[cfg(feature = "hot_reload")]
use dioxus_core::{Template, TemplateAttribute, TemplateNode};
#[cfg(feature = "hot_reload")]
pub use hot_reload::HotReloadingContext;
#[cfg(feature = "hot_reload")]
use internment::Intern;

// imports
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use renderer::TemplateRenderer;
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

        // Empty templates just are placeholders for "none"
        if self.roots.is_empty() {
            return quote! { None };
        }

        quote! {
            Some({ #body })
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

impl ToTokens for CallBody {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let body: TemplateRenderer = TemplateRenderer {
            roots: &self.roots,
            location: None,
        };

        // Empty templates just are placeholders for "none"
        if self.roots.is_empty() {
            return out_tokens.append_all(quote! { None });
        }

        out_tokens.append_all(quote! {
            Some({ #body })
        })
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
mod tests {
    use super::*;

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

    #[test]
    fn diff_uses_for() {}

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
}
