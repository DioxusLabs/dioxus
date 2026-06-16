use crate::innerlude::*;
use proc_macro2::{Span, TokenStream as TokenStream2};
use proc_macro2_diagnostics::SpanDiagnosticExt;
use quote::{ToTokens, TokenStreamExt, quote};
use std::fmt::{Display, Formatter};
use syn::{
    Ident, LitStr, Result, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Brace,
};

/// Parse the VNode::Element type
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Element {
    /// div { } -> div
    pub name: ElementName,

    /// The actual attributes that were parsed
    pub raw_attributes: Vec<Attribute>,

    /// The attributes after merging - basically the formatted version of the combined attributes
    /// where possible.
    ///
    /// These are the actual attributes that get rendered out
    pub merged_attributes: Vec<Attribute>,

    /// The `...` spread attributes.
    pub spreads: Vec<Spread>,

    // /// Elements can have multiple, unlike components which can only have one
    // pub spreads: Vec<Spread>,
    /// The children of the element
    pub children: Vec<BodyNode>,

    /// the brace of the `div { }`
    pub brace: Option<Brace>,

    /// A list of diagnostics that were generated during parsing. This element might be a valid rsx_block
    /// but not technically a valid element - these diagnostics tell us what's wrong and then are used
    /// when rendering
    pub diagnostics: Diagnostics,
}

impl Parse for Element {
    fn parse(stream: ParseStream) -> Result<Self> {
        let name = stream.parse::<ElementName>()?;

        // We very liberally parse elements - they might not even have a brace!
        // This is designed such that we can emit a diagnostic instead of failing to parse.
        let mut brace = None;
        let mut block = RsxBlock::default();

        match stream.peek(Brace) {
            // If the element is followed by a brace, it is complete. Parse the body
            true => {
                block = stream.parse::<RsxBlock>()?;
                brace = Some(block.brace);
            }

            // Otherwise, it is incomplete. Add a diagnostic
            false => block.diagnostics.push(
                name.span()
                    .error("Elements must be followed by braces")
                    .help("Did you forget a brace?"),
            ),
        }

        // Make sure these attributes have element context for name and namespace resolution.
        for attr in block.attributes.iter_mut() {
            attr.el_name = Some(name.clone());
        }

        // Assemble the new element from the contents of the block
        let mut element = Element {
            brace,
            name: name.clone(),
            raw_attributes: block.attributes,
            children: block.children,
            diagnostics: block.diagnostics,
            spreads: block.spreads.clone(),
            merged_attributes: Vec::new(),
        };

        // And then merge the various attributes together
        // The original raw_attributes are kept for lossless parsing used by hotreload/autofmt
        element.merge_attributes();

        // And then merge the spreads *after* the attributes are merged. This ensures walking the
        // merged attributes in path order stops before we hit the spreads, but spreads are still
        // counted as dynamic attributes
        for spread in block.spreads.iter() {
            element.merged_attributes.push(Attribute {
                name: AttributeName::Spread(spread.dots),
                colon: None,
                value: AttributeValue::AttrExpr(PartialExpr::from_expr(&spread.expr)),
                comma: spread.comma,
                el_name: Some(name.clone()),
            });
        }

        Ok(element)
    }
}

impl ToTokens for Element {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let body = TemplateBody {
            roots: vec![BodyNode::Element(self.clone())],
            template_idx: Default::default(),
            diagnostics: Diagnostics::new(),
        };
        let template = crate::flat_template::FlatTemplatePieces::from_body(&body);
        let definitions = template.definitions();
        let view = template.view_expr();

        tokens.append_all(quote! {
            {
                #(#definitions)*
                #view
            }
        });
    }
}

impl Element {
    pub(crate) fn add_merging_non_string_diagnostic(diagnostics: &mut Diagnostics, span: Span) {
        diagnostics.push(span.error("Cannot merge non-fmt literals").help(
            "Only formatted strings can be merged together. If you want to merge literals, you can use a format string.",
        ));
    }

    /// Collapses ifmt attributes into a single dynamic attribute using a space or `;` as a delimiter
    ///
    /// ```ignore,
    /// div {
    ///     class: "abc-def",
    ///     class: if some_expr { "abc" },
    /// }
    /// ```
    fn merge_attributes(&mut self) {
        let mut attrs: Vec<&Attribute> = vec![];

        for attr in &self.raw_attributes {
            if attrs.iter().any(|old_attr| old_attr.name == attr.name) {
                continue;
            }

            attrs.push(attr);
        }

        for attr in attrs {
            if attr.name.is_likely_key() {
                continue;
            }

            // Collect all the attributes with the same name
            let matching_attrs = self
                .raw_attributes
                .iter()
                .filter(|a| a.name == attr.name)
                .collect::<Vec<_>>();

            // if there's only one attribute with this name, then we don't need to merge anything
            if matching_attrs.len() == 1 {
                self.merged_attributes.push(attr.clone());
                continue;
            }

            // If there are multiple attributes with the same name, then we need to merge them
            // This will be done by creating an ifmt attribute that combines all the segments
            // We might want to throw a diagnostic of trying to merge things together that might not
            // make a whole lot of sense - like merging two exprs together
            let mut out = IfmtInput::new(attr.span());

            for (idx, matching_attr) in matching_attrs.iter().enumerate() {
                // If this is the first attribute, then we don't need to add a delimiter
                if idx != 0 {
                    // FIXME: I don't want to special case anything - but our delimiter is special cased to a space
                    // We really don't want to special case anything in the macro, but the hope here is that
                    // multiline strings can be merged with a space
                    out.push_raw_str(" ".to_string());
                }

                // Merge raw literals into the output
                if let AttributeValue::AttrLiteral(HotLiteral::Fmted(lit)) = &matching_attr.value {
                    out.push_ifmt(lit.formatted_input.clone());
                    continue;
                }

                // Merge `if cond { "abc" } else if ...` into the output
                if let AttributeValue::IfExpr(value) = &matching_attr.value {
                    out.push_expr(value.quote_as_string(&mut self.diagnostics));
                    continue;
                }

                Self::add_merging_non_string_diagnostic(
                    &mut self.diagnostics,
                    matching_attr.span(),
                );
            }

            let out_lit = HotLiteral::Fmted(out.into());

            self.merged_attributes.push(Attribute {
                name: attr.name.clone(),
                value: AttributeValue::AttrLiteral(out_lit),
                colon: attr.colon,
                comma: matching_attrs.last().unwrap().comma,
                el_name: attr.el_name.clone(),
            });
        }
    }

    pub(crate) fn key(&self) -> Option<&AttributeValue> {
        self.raw_attributes
            .iter()
            .find(|attr| attr.name.is_likely_key())
            .map(|attr| &attr.value)
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum ElementName {
    Ident(Ident),
    Custom(LitStr),
}

impl ToTokens for ElementName {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            ElementName::Ident(i) => tokens.append_all(quote! { #i }),
            ElementName::Custom(s) => s.to_tokens(tokens),
        }
    }
}

impl Parse for ElementName {
    fn parse(stream: ParseStream) -> Result<Self> {
        let raw =
            Punctuated::<Ident, Token![-]>::parse_separated_nonempty_with(stream, parse_raw_ident)?;
        if raw.len() == 1 {
            Ok(ElementName::Ident(raw.into_iter().next().unwrap()))
        } else {
            let span = raw.span();
            let tag = raw
                .into_iter()
                .map(|ident| ident.to_string())
                .collect::<Vec<_>>()
                .join("-");
            let tag = LitStr::new(&tag, span);
            Ok(ElementName::Custom(tag))
        }
    }
}

impl ElementName {
    pub(crate) fn tag_name_string(&self) -> String {
        match self {
            ElementName::Ident(i) => normalize_element_ident(i),
            ElementName::Custom(s) => s.value(),
        }
    }

    pub(crate) fn namespace(&self) -> Option<&'static str> {
        namespace_for_tag(self.tag_name_string().as_str())
    }

    pub(crate) fn tag_name(&self) -> TokenStream2 {
        match self {
            ElementName::Ident(_) => {
                let name = self.tag_name_string();
                quote! { #name }
            }
            ElementName::Custom(s) => quote! { #s },
        }
    }

    pub fn span(&self) -> Span {
        match self {
            ElementName::Ident(i) => i.span(),
            ElementName::Custom(s) => s.span(),
        }
    }
}

fn normalize_element_ident(ident: &Ident) -> String {
    match ident.to_string().strip_prefix("r#") {
        Some(raw) => raw.to_string(),
        None => match ident.to_string().as_str() {
            "annotationXml" => "annotation-xml".to_string(),
            name => name.to_string(),
        },
    }
}

fn namespace_for_tag(tag: &str) -> Option<&'static str> {
    match tag {
        "svg"
        | "animate"
        | "animateMotion"
        | "animateTransform"
        | "circle"
        | "clipPath"
        | "defs"
        | "desc"
        | "discard"
        | "ellipse"
        | "feBlend"
        | "feColorMatrix"
        | "feComponentTransfer"
        | "feComposite"
        | "feConvolveMatrix"
        | "feDiffuseLighting"
        | "feDisplacementMap"
        | "feDistantLight"
        | "feDropShadow"
        | "feFlood"
        | "feFuncA"
        | "feFuncB"
        | "feFuncG"
        | "feFuncR"
        | "feGaussianBlur"
        | "feImage"
        | "feMerge"
        | "feMergeNode"
        | "feMorphology"
        | "feOffset"
        | "fePointLight"
        | "feSpecularLighting"
        | "feSpotLight"
        | "feTile"
        | "feTurbulence"
        | "filter"
        | "foreignObject"
        | "g"
        | "hatch"
        | "hatchpath"
        | "image"
        | "line"
        | "linearGradient"
        | "marker"
        | "mask"
        | "metadata"
        | "mpath"
        | "path"
        | "pattern"
        | "polygon"
        | "polyline"
        | "radialGradient"
        | "rect"
        | "set"
        | "stop"
        | "switch"
        | "symbol"
        | "text"
        | "textPath"
        | "tspan"
        | "use"
        | "view" => Some("http://www.w3.org/2000/svg"),
        "annotation" | "annotation-xml" | "merror" | "math" | "mfrac" | "mi" | "mmultiscripts"
        | "mn" | "mo" | "mover" | "mpadded" | "mprescripts" | "mroot" | "mrow" | "ms"
        | "mspace" | "msqrt" | "mstyle" | "msub" | "msubsup" | "msup" | "mtable" | "mtd"
        | "mtext" | "mtr" | "munder" | "munderover" | "semantics" => {
            Some("http://www.w3.org/1998/Math/MathML")
        }
        _ => None,
    }
}

impl PartialEq<&str> for ElementName {
    fn eq(&self, other: &&str) -> bool {
        match self {
            ElementName::Ident(i) => i == *other,
            ElementName::Custom(s) => s.value() == *other,
        }
    }
}

impl Display for ElementName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ElementName::Ident(i) => write!(f, "{}", i),
            ElementName::Custom(s) => write!(f, "{}", s.value()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prettier_please::PrettyUnparse;

    #[test]
    fn parses_name() {
        let _parsed: ElementName = syn::parse2(quote::quote! { div }).unwrap();
        let _parsed: ElementName = syn::parse2(quote::quote! { some-cool-element }).unwrap();

        let _parsed: Element = syn::parse2(quote::quote! { div {} }).unwrap();
        let _parsed: Element = syn::parse2(quote::quote! { some-cool-element {} }).unwrap();

        let parsed: Element = syn::parse2(quote::quote! {
            some-cool-div {
                id: "hi",
                id: "hi {abc}",
                id: "hi {def}",
                class: 123,
                something: bool,
                data_attr: "data",
                data_attr: "data2",
                data_attr: "data3",
                exp: { some_expr },
                something: {cool},
                something: bool,
                something: 123,
                onclick: move |_| {
                    println!("hello world");
                },
                "some-attr": "hello world",
                onclick: move |_| {},
                class: "hello world",
                id: "my-id",
                data_attr: "data",
                data_attr: "data2",
                data_attr: "data3",
                "somte_attr3": "hello world",
                something: {cool},
                something: bool,
                something: 123,
                onclick: move |_| {
                    println!("hello world");
                },
                ..attrs1,
                ..attrs2,
                ..attrs3
            }
        })
        .unwrap();

        dbg!(parsed);
    }

    #[test]
    fn parses_variety() {
        let input = quote::quote! {
            div {
                class: "hello world",
                id: "my-id",
                data_attr: "data",
                data_attr: "data2",
                data_attr: "data3",
                "somte_attr3": "hello world",
                something: {cool},
                something: bool,
                something: 123,
                onclick: move |_| {
                    println!("hello world");
                },
                ..attrs,
                ..attrs2,
                ..attrs3
            }
        };

        let parsed: Element = syn::parse2(input).unwrap();
        dbg!(parsed);
    }

    #[test]
    fn to_tokens_properly() {
        let input = quote::quote! {
            div {
                class: "hello world",
                class2: "hello {world}",
                class3: "goodbye {world}",
                class4: "goodbye world",
                "something": "cool {blah}",
                "something2": "cooler",
                div {
                    div {
                        h1 { class: "h1 col" }
                        h2 { class: "h2 col" }
                        h3 { class: "h3 col" }
                        div {}
                    }
                }
            }
        };

        let parsed: Element = syn::parse2(input).unwrap();
        println!("{}", parsed.to_token_stream().pretty_unparse());
    }

    #[test]
    fn to_tokens_with_diagnostic() {
        let input = quote::quote! {
            div {
                class: "hello world",
                id: "my-id",
                ..attrs,
                div {
                    ..attrs,
                    class: "hello world",
                    id: "my-id",
                }
            }
        };

        let parsed: Element = syn::parse2(input).unwrap();
        println!("{}", parsed.to_token_stream().pretty_unparse());
    }

    #[test]
    fn merge_trivial_attributes() {
        let input = quote::quote! {
            div {
                class: "foo",
                class: "bar",
            }
        };

        let parsed: Element = syn::parse2(input).unwrap();
        assert_eq!(parsed.diagnostics.len(), 0);
        assert_eq!(parsed.merged_attributes.len(), 1);
        assert_eq!(
            parsed.merged_attributes[0].name.to_string(),
            "class".to_string()
        );

        let attr = &parsed.merged_attributes[0].value;

        assert_eq!(
            attr.to_token_stream().pretty_unparse().as_str(),
            "\"foo bar\""
        );

        if let AttributeValue::AttrLiteral(_) = attr {
        } else {
            panic!("expected literal")
        }
    }

    #[test]
    fn merge_formatted_attributes() {
        let input = quote::quote! {
            div {
                class: "foo",
                class: "{bar}",
            }
        };

        let parsed: Element = syn::parse2(input).unwrap();
        assert_eq!(parsed.diagnostics.len(), 0);
        assert_eq!(parsed.merged_attributes.len(), 1);
        assert_eq!(
            parsed.merged_attributes[0].name.to_string(),
            "class".to_string()
        );

        let attr = &parsed.merged_attributes[0].value;

        assert_eq!(
            attr.to_token_stream().pretty_unparse().as_str(),
            "::std::format!(\"foo {0:}\", bar)"
        );

        if let AttributeValue::AttrLiteral(_) = attr {
        } else {
            panic!("expected literal")
        }
    }

    #[test]
    fn merge_conditional_attributes() {
        let input = quote::quote! {
            div {
                class: "foo",
                class: if true { "bar" },
                class: if false { "baz" } else { "qux" }
            }
        };

        let parsed: Element = syn::parse2(input).unwrap();
        assert_eq!(parsed.diagnostics.len(), 0);
        assert_eq!(parsed.merged_attributes.len(), 1);
        assert_eq!(
            parsed.merged_attributes[0].name.to_string(),
            "class".to_string()
        );

        let attr = &parsed.merged_attributes[0].value;

        assert_eq!(
            attr.to_token_stream().pretty_unparse().as_str(),
            "::std::format!(\n    \
                \"foo {0:} {1:}\",\n    \
                { if true { \"bar\".to_string() } else { ::std::string::String::new() } },\n    \
                { if false { \"baz\".to_string() } else { \"qux\".to_string() } },\n\
            )"
        );

        if let AttributeValue::AttrLiteral(_) = attr {
        } else {
            panic!("expected literal")
        }
    }

    #[test]
    fn merge_all_attributes() {
        let input = quote::quote! {
            div {
                class: "foo",
                class: "{bar}",
                class: if true { "baz" },
                class: if false { "{qux}" } else { "quux" }
            }
        };

        let parsed: Element = syn::parse2(input).unwrap();
        assert_eq!(parsed.diagnostics.len(), 0);
        assert_eq!(parsed.merged_attributes.len(), 1);
        assert_eq!(
            parsed.merged_attributes[0].name.to_string(),
            "class".to_string()
        );

        let attr = &parsed.merged_attributes[0].value;

        if cfg!(debug_assertions) {
            assert_eq!(
                attr.to_token_stream().pretty_unparse().as_str(),
                "::std::format!(\n    \
                    \"foo {0:} {1:} {2:}\",\n    \
                    bar,\n    \
                    { if true { \"baz\".to_string() } else { ::std::string::String::new() } },\n    \
                    { if false { ::std::format!(\"{qux}\").to_string() } else { \"quux\".to_string() } },\n\
                )"
            );
        } else {
            assert_eq!(
                attr.to_token_stream().pretty_unparse().as_str(),
                "::std::format!(\n    \
                    \"foo {0:} {1:} {2:}\",\n    \
                    bar,\n    \
                    { if true { \"baz\".to_string() } else { ::std::string::String::new() } },\n    \
                    { if false { (qux).to_string().to_string() } else { \"quux\".to_string() } },\n\
                )"
            );
        }

        if let AttributeValue::AttrLiteral(_) = attr {
        } else {
            panic!("expected literal")
        }
    }

    /// There are a number of cases where merging attributes doesn't make sense
    /// - merging two expressions together
    /// - merging two literals together
    /// - merging a literal and an expression together
    ///
    /// etc
    ///
    /// We really only want to merge formatted things together
    ///
    /// IE
    /// class: "hello world ",
    /// class: if some_expr { "abc" }
    ///
    /// Some open questions - should the delimiter be explicit?
    #[test]
    fn merging_weird_fails() {
        let input = quote::quote! {
            div {
                class: "hello world",
                class: if some_expr { 123 },

                style: "color: red;",
                style: "color: blue;",

                width: "1px",
                width: 1,
                width: false,
                contenteditable: true,
            }
        };

        let parsed: Element = syn::parse2(input).unwrap();

        assert_eq!(parsed.merged_attributes.len(), 4);
        assert_eq!(parsed.diagnostics.len(), 3);

        // style should not generate a diagnostic
        assert!(
            !parsed
                .diagnostics
                .diagnostics
                .into_iter()
                .any(|f| f.emit_as_item_tokens().to_string().contains("style"))
        );
    }

    #[test]
    fn diagnostics() {
        let input = quote::quote! {
            p {
                class: "foo bar"
                "Hello world"
            }
        };

        let _parsed: Element = syn::parse2(input).unwrap();
    }

    #[test]
    fn parses_raw_elements() {
        let input = quote::quote! {
            use {
                "hello"
            }
        };

        let _parsed: Element = syn::parse2(input).unwrap();
    }
}
