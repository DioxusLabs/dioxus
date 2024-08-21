use crate::innerlude::*;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::ToTokens;
use syn::{
    ext::IdentExt,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token::{self},
    Ident, LitStr, Result, Token,
};

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum BodyNode {
    /// div {}
    Element(Element),

    /// Component {}
    Component(Component),

    /// "text {formatted}"
    Text(TextNode),

    /// {expr}
    RawExpr(ExprNode),

    /// for item in items {}
    ForLoop(ForLoop),

    /// if cond {} else if cond {} (else {}?)
    IfChain(IfChain),
}

impl Parse for BodyNode {
    fn parse(stream: ParseStream) -> Result<Self> {
        if stream.peek(LitStr) {
            return Ok(BodyNode::Text(stream.parse()?));
        }

        // Transform for loops into into_iter calls
        if stream.peek(Token![for]) {
            return Ok(BodyNode::ForLoop(stream.parse()?));
        }

        // Transform unterminated if statements into terminated optional if statements
        if stream.peek(Token![if]) {
            return Ok(BodyNode::IfChain(stream.parse()?));
        }

        // Match statements are special but have no special arm syntax
        // we could allow arm syntax if we wanted.
        //
        // And it might even backwards compatible? - I think it is with the right fallback
        // -> parse as bodynode (BracedRawExpr will kick in on multiline arms)
        // -> if that fails parse as an expr, since that arm might be a one-liner
        //
        // ```
        // match expr {
        //    val => rsx! { div {} },
        //    other_val => rsx! { div {} }
        // }
        // ```
        if stream.peek(Token![match]) {
            return Ok(BodyNode::RawExpr(stream.parse()?));
        }

        // Raw expressions need to be wrapped in braces - let RawBracedExpr handle partial expansion
        if stream.peek(token::Brace) {
            return Ok(BodyNode::RawExpr(stream.parse()?));
        }

        // If there's an ident immediately followed by a dash, it's a web component
        // Web components support no namespacing, so just parse it as an element directly
        if stream.peek(Ident::peek_any) && stream.peek2(Token![-]) {
            return Ok(BodyNode::Element(stream.parse::<Element>()?));
        }

        // this is an Element if the path is:
        //
        // - one ident
        // - 1st char is lowercase
        // - no underscores (reserved for components)
        // And it is not:
        // - the start of a path with components
        //
        // example:
        // div {}
        if stream.peek(Ident::peek_any) && !stream.peek2(Token![::]) {
            let ident = parse_raw_ident(&stream.fork()).unwrap();
            let el_name = ident.to_string();
            let first_char = el_name.chars().next().unwrap();

            if first_char.is_ascii_lowercase() && !el_name.contains('_') {
                return Ok(BodyNode::Element(stream.parse::<Element>()?));
            }
        }

        // Otherwise this should be Component, allowed syntax:
        // - syn::Path
        // - PathArguments can only apper in last segment
        // - followed by `{` or `(`, note `(` cannot be used with one ident
        //
        // example
        // Div {}
        // ::Div {}
        // crate::Div {}
        // component {} <-- already handled by elements
        // ::component {}
        // crate::component{}
        // Input::<InputProps<'_, i32> {}
        // crate::Input::<InputProps<'_, i32> {}
        Ok(BodyNode::Component(stream.parse()?))
    }
}

impl ToTokens for BodyNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            BodyNode::Element(ela) => ela.to_tokens(tokens),
            BodyNode::RawExpr(exp) => exp.to_tokens(tokens),
            BodyNode::Text(txt) => txt.to_tokens(tokens),
            BodyNode::ForLoop(floop) => floop.to_tokens(tokens),
            BodyNode::Component(comp) => comp.to_tokens(tokens),
            BodyNode::IfChain(ifchain) => ifchain.to_tokens(tokens),
        }
    }
}

impl BodyNode {
    /// Convert this BodyNode into a TemplateNode.
    ///
    /// dioxus-core uses this to understand templates at compiletime
    #[cfg(feature = "hot_reload")]
    pub fn to_template_node<Ctx: dioxus_core_types::HotReloadingContext>(
        &self,
    ) -> dioxus_core::TemplateNode {
        use dioxus_core::TemplateNode;
        match self {
            BodyNode::Element(el) => {
                let rust_name = el.name.to_string();

                let (tag, namespace) =
                    Ctx::map_element(&rust_name).unwrap_or((intern(rust_name.as_str()), None));

                TemplateNode::Element {
                    tag,
                    namespace,
                    children: intern(
                        el.children
                            .iter()
                            .map(|c| c.to_template_node::<Ctx>())
                            .collect::<Vec<_>>(),
                    ),
                    attrs: intern(
                        el.merged_attributes
                            .iter()
                            .map(|attr| attr.to_template_attribute::<Ctx>())
                            .collect::<Vec<_>>(),
                    ),
                }
            }
            BodyNode::Text(text) => text.to_template_node(),
            BodyNode::RawExpr(exp) => TemplateNode::Dynamic {
                id: exp.dyn_idx.get(),
            },
            BodyNode::Component(comp) => TemplateNode::Dynamic {
                id: comp.dyn_idx.get(),
            },
            BodyNode::ForLoop(floop) => TemplateNode::Dynamic {
                id: floop.dyn_idx.get(),
            },
            BodyNode::IfChain(chain) => TemplateNode::Dynamic {
                id: chain.dyn_idx.get(),
            },
        }
    }

    pub fn get_dyn_idx(&self) -> usize {
        match self {
            BodyNode::Text(text) => text.dyn_idx.get(),
            BodyNode::RawExpr(exp) => exp.dyn_idx.get(),
            BodyNode::Component(comp) => comp.dyn_idx.get(),
            BodyNode::ForLoop(floop) => floop.dyn_idx.get(),
            BodyNode::IfChain(chain) => chain.dyn_idx.get(),
            BodyNode::Element(_) => panic!("Cannot get dyn_idx for this node"),
        }
    }

    pub fn set_dyn_idx(&self, idx: usize) {
        match self {
            BodyNode::Text(text) => text.dyn_idx.set(idx),
            BodyNode::RawExpr(exp) => exp.dyn_idx.set(idx),
            BodyNode::Component(comp) => comp.dyn_idx.set(idx),
            BodyNode::ForLoop(floop) => floop.dyn_idx.set(idx),
            BodyNode::IfChain(chain) => chain.dyn_idx.set(idx),
            BodyNode::Element(_) => panic!("Cannot set dyn_idx for this node"),
        }
    }

    pub fn is_litstr(&self) -> bool {
        matches!(self, BodyNode::Text { .. })
    }

    pub fn span(&self) -> Span {
        match self {
            BodyNode::Element(el) => el.name.span(),
            BodyNode::Component(component) => component.name.span(),
            BodyNode::Text(text) => text.input.span(),
            BodyNode::RawExpr(exp) => exp.span(),
            BodyNode::ForLoop(fl) => fl.for_token.span(),
            BodyNode::IfChain(f) => f.if_token.span(),
        }
    }

    pub fn element_children(&self) -> &[BodyNode] {
        match self {
            BodyNode::Element(el) => &el.children,
            _ => panic!("Children not available for this node"),
        }
    }

    pub fn el_name(&self) -> &ElementName {
        match self {
            BodyNode::Element(el) => &el.name,
            _ => panic!("Element name not available for this node"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn parsing_matches() {
        let element = quote! { div { class: "inline-block mr-4", icons::icon_14 {} } };
        assert!(matches!(
            syn::parse2::<BodyNode>(element).unwrap(),
            BodyNode::Element(_)
        ));

        let text = quote! { "Hello, world!" };
        assert!(matches!(
            syn::parse2::<BodyNode>(text).unwrap(),
            BodyNode::Text(_)
        ));

        let component = quote! { Component {} };
        assert!(matches!(
            syn::parse2::<BodyNode>(component).unwrap(),
            BodyNode::Component(_)
        ));

        let raw_expr = quote! { { 1 + 1 } };
        assert!(matches!(
            syn::parse2::<BodyNode>(raw_expr).unwrap(),
            BodyNode::RawExpr(_)
        ));

        let for_loop = quote! { for item in items {} };
        assert!(matches!(
            syn::parse2::<BodyNode>(for_loop).unwrap(),
            BodyNode::ForLoop(_)
        ));

        let if_chain = quote! { if cond {} else if cond {} };
        assert!(matches!(
            syn::parse2::<BodyNode>(if_chain).unwrap(),
            BodyNode::IfChain(_)
        ));

        let match_expr = quote! {
            match blah {
                val => rsx! { div {} },
                other_val => rsx! { div {} }
            }
        };
        assert!(matches!(
            syn::parse2::<BodyNode>(match_expr).unwrap(),
            BodyNode::RawExpr(_)
        ),);

        let incomplete_component = quote! {
            some::cool::Component
        };
        assert!(matches!(
            syn::parse2::<BodyNode>(incomplete_component).unwrap(),
            BodyNode::Component(_)
        ),);
    }
}
