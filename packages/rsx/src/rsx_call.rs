//! The actual rsx! macro implementation.
//!
//! This mostly just defers to the root TemplateBody with some additional tooling to provide better errors.
//! Currently the additional tooling doesn't do much.

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use std::{cell::Cell, fmt::Debug};
use syn::{
    parse::{Parse, ParseStream},
    Result,
};

use crate::{BodyNode, TemplateBody};

/// The Callbody is the contents of the rsx! macro
///
/// It is a list of BodyNodes, which are the different parts of the template.
/// The Callbody contains no information about how the template will be rendered, only information about the parsed tokens.
///
/// Every callbody should be valid, so you can use it to build a template.
/// To generate the code used to render the template, use the ToTokens impl on the Callbody, or with the `render_with_location` method.
///
/// Ideally we don't need the metadata here and can bake the idx-es into the templates themselves but I haven't figured out how to do that yet.
#[derive(Debug, Clone)]
pub struct CallBody {
    pub body: TemplateBody,
    pub ifmt_idx: Cell<usize>,
    pub template_idx: Cell<usize>,
}

impl Parse for CallBody {
    fn parse(input: ParseStream) -> Result<Self> {
        // Defer to the `new` method such that we can wire up hotreload information
        Ok(CallBody::new(input.parse()?))
    }
}

impl ToTokens for CallBody {
    fn to_tokens(&self, out: &mut TokenStream2) {
        match self.body.is_empty() {
            true => quote! { dioxus_core::VNode::empty() }.to_tokens(out),
            false => self.body.to_tokens(out),
        }
    }
}

impl CallBody {
    /// Create a new CallBody from a TemplateBody
    ///
    /// This will overwrite all internal metadata regarding hotreloading.
    pub fn new(body: TemplateBody) -> Self {
        let body = CallBody {
            body,
            ifmt_idx: Cell::new(0),
            template_idx: Cell::new(0),
        };

        body.body.template_idx.set(body.next_template_idx());

        body.cascade_hotreload_info(&body.body.roots);

        body
    }

    /// Parse a stream into a CallBody. Return all error immediately instead of trying to partially expand the macro
    ///
    /// This should be preferred over `parse` if you are outside of a macro
    pub fn parse_strict(input: ParseStream) -> Result<Self> {
        // todo: actually throw warnings if there are any
        Self::parse(input)
    }

    /// With the entire knowledge of the macro call, wire up location information for anything hotreloading
    /// specific. It's a little bit simpler just to have a global id per callbody than to try and track it
    /// relative to each template, though we could do that if we wanted to.
    ///
    /// For now this is just information for ifmts and templates so that when they generate, they can be
    /// tracked back to the original location in the source code, to support formatted string hotreloading.
    ///
    /// Note that there are some more complex cases we could in theory support, but have bigger plans
    /// to enable just pure rust hotreloading that would make those tricks moot. So, manage more of
    /// the simple cases until that proper stuff ships.
    ///
    /// We need to make sure to wire up:
    /// - subtemplate IDs
    /// - ifmt IDs
    /// - dynamic node IDs
    /// - dynamic attribute IDs
    /// - paths for dynamic nodes and attributes
    ///
    /// Lots of wiring!
    ///
    /// However, here, we only need to wire up ifmt and template IDs since TemplateBody will handle the rest.
    ///
    /// This is better though since we can save the relevant data on the structures themselves.
    fn cascade_hotreload_info(&self, nodes: &[BodyNode]) {
        for node in nodes.iter() {
            match node {
                BodyNode::RawExpr(_) => { /* one day maybe provide hr here?*/ }

                BodyNode::Text(text) => {
                    // one day we could also provide HR here to allow dynamic parts on the fly
                    if !text.is_static() {
                        text.hr_idx.set(self.next_ifmt_idx());
                    }
                }

                BodyNode::Element(el) => {
                    // Walk the attributes looking for hotreload opportunities
                    for attr in &el.merged_attributes {
                        attr.with_literal(|lit| lit.hr_idx.set(self.next_ifmt_idx()));
                    }

                    self.cascade_hotreload_info(&el.children);
                }

                BodyNode::Component(comp) => {
                    // walk the props looking for hotreload opportunities
                    for prop in comp.fields.iter() {
                        prop.with_literal(|lit| lit.hr_idx.set(self.next_ifmt_idx()));
                    }

                    comp.children.template_idx.set(self.next_template_idx());
                    self.cascade_hotreload_info(&comp.children.roots);
                }

                BodyNode::ForLoop(floop) => {
                    floop.body.template_idx.set(self.next_template_idx());
                    self.cascade_hotreload_info(&floop.body.roots);
                }

                BodyNode::IfChain(chain) => chain.for_each_branch(&mut |body| {
                    body.template_idx.set(self.next_template_idx());
                    self.cascade_hotreload_info(&body.roots)
                }),
            }
        }
    }

    fn next_ifmt_idx(&self) -> usize {
        let idx = self.ifmt_idx.get();
        self.ifmt_idx.set(idx + 1);
        idx
    }

    fn next_template_idx(&self) -> usize {
        let idx = self.template_idx.get();
        self.template_idx.set(idx + 1);
        idx
    }
}
