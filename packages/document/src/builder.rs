//! Document element helpers for the builder API.
//!
//! This module provides ergonomic helpers for creating document head elements
//! like title, stylesheet links, and meta tags.
//!
//! # Example
//!
//! ```rust,ignore
//! use dioxus::prelude::*;
//!
//! fn app() -> Element {
//!     fragment()
//!         .child(doc_title("My App"))
//!         .child(doc_stylesheet("/assets/style.css"))
//!         .child(doc_meta().name("viewport").content("width=device-width").build())
//!         .child(body_content())
//!         .build()
//! }
//! ```

use dioxus_core::Element;
use dioxus_html::builder::text_node;

use crate::{self as document, LinkProps, MetaProps, TitleProps};

/// Create a document title element with the given text.
///
/// # Example
///
/// ```rust,ignore
/// doc_title("My Page Title")
/// ```
pub fn doc_title(text: impl ToString) -> Element {
    document::Title(TitleProps::builder().children(text_node(text)).build())
}

/// Create a stylesheet link element.
///
/// # Example
///
/// ```rust,ignore
/// doc_stylesheet("/assets/style.css")
/// ```
pub fn doc_stylesheet(href: impl ToString) -> Element {
    document::Stylesheet(
        LinkProps::builder()
            .rel(Some("stylesheet".to_string()))
            .r#type(Some("text/css".to_string()))
            .href(Some(href.to_string()))
            .build(),
    )
}

/// Builder for document meta tags.
///
/// # Example
///
/// ```rust,ignore
/// doc_meta()
///     .name("viewport")
///     .content("width=device-width, initial-scale=1")
///     .build()
///
/// doc_meta()
///     .charset("utf-8")
///     .build()
///
/// doc_meta()
///     .property("og:title")
///     .content("My Page")
///     .build()
/// ```
#[derive(Default)]
pub struct DocMetaBuilder {
    property: Option<String>,
    name: Option<String>,
    charset: Option<String>,
    http_equiv: Option<String>,
    content: Option<String>,
    data: Option<String>,
}

impl DocMetaBuilder {
    /// Create a new meta tag builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the property attribute (for Open Graph tags).
    pub fn property(mut self, value: impl ToString) -> Self {
        self.property = Some(value.to_string());
        self
    }

    /// Set the name attribute.
    pub fn name(mut self, value: impl ToString) -> Self {
        self.name = Some(value.to_string());
        self
    }

    /// Set the charset attribute.
    pub fn charset(mut self, value: impl ToString) -> Self {
        self.charset = Some(value.to_string());
        self
    }

    /// Set the http-equiv attribute.
    pub fn http_equiv(mut self, value: impl ToString) -> Self {
        self.http_equiv = Some(value.to_string());
        self
    }

    /// Set the content attribute.
    pub fn content(mut self, value: impl ToString) -> Self {
        self.content = Some(value.to_string());
        self
    }

    /// Set the data attribute.
    pub fn data(mut self, value: impl ToString) -> Self {
        self.data = Some(value.to_string());
        self
    }

    /// Build the meta element.
    pub fn build(self) -> Element {
        document::Meta(
            MetaProps::builder()
                .property(self.property)
                .name(self.name)
                .charset(self.charset)
                .http_equiv(self.http_equiv)
                .content(self.content)
                .data(self.data)
                .build(),
        )
    }
}

/// Create a new document meta tag builder.
///
/// # Example
///
/// ```rust,ignore
/// doc_meta()
///     .name("description")
///     .content("A great page")
///     .build()
/// ```
pub fn doc_meta() -> DocMetaBuilder {
    DocMetaBuilder::new()
}

/// Builder for document link elements.
///
/// # Example
///
/// ```rust,ignore
/// doc_link()
///     .rel("icon")
///     .href("/favicon.ico")
///     .build()
/// ```
#[derive(Default)]
pub struct DocLinkBuilder {
    rel: Option<String>,
    href: Option<String>,
    media: Option<String>,
    title: Option<String>,
    disabled: Option<bool>,
    r#as: Option<String>,
    sizes: Option<String>,
    crossorigin: Option<String>,
    referrerpolicy: Option<String>,
    fetchpriority: Option<String>,
    hreflang: Option<String>,
    integrity: Option<String>,
    r#type: Option<String>,
    blocking: Option<String>,
}

impl DocLinkBuilder {
    /// Create a new link builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the rel attribute.
    pub fn rel(mut self, value: impl ToString) -> Self {
        self.rel = Some(value.to_string());
        self
    }

    /// Set the href attribute.
    pub fn href(mut self, value: impl ToString) -> Self {
        self.href = Some(value.to_string());
        self
    }

    /// Set the media attribute.
    pub fn media(mut self, value: impl ToString) -> Self {
        self.media = Some(value.to_string());
        self
    }

    /// Set the title attribute.
    pub fn title(mut self, value: impl ToString) -> Self {
        self.title = Some(value.to_string());
        self
    }

    /// Set the disabled attribute.
    pub fn disabled(mut self, value: bool) -> Self {
        self.disabled = Some(value);
        self
    }

    /// Set the as attribute (for preloading).
    pub fn r#as(mut self, value: impl ToString) -> Self {
        self.r#as = Some(value.to_string());
        self
    }

    /// Set the sizes attribute.
    pub fn sizes(mut self, value: impl ToString) -> Self {
        self.sizes = Some(value.to_string());
        self
    }

    /// Set the crossorigin attribute.
    pub fn crossorigin(mut self, value: impl ToString) -> Self {
        self.crossorigin = Some(value.to_string());
        self
    }

    /// Set the referrerpolicy attribute.
    pub fn referrerpolicy(mut self, value: impl ToString) -> Self {
        self.referrerpolicy = Some(value.to_string());
        self
    }

    /// Set the fetchpriority attribute.
    pub fn fetchpriority(mut self, value: impl ToString) -> Self {
        self.fetchpriority = Some(value.to_string());
        self
    }

    /// Set the hreflang attribute.
    pub fn hreflang(mut self, value: impl ToString) -> Self {
        self.hreflang = Some(value.to_string());
        self
    }

    /// Set the integrity attribute.
    pub fn integrity(mut self, value: impl ToString) -> Self {
        self.integrity = Some(value.to_string());
        self
    }

    /// Set the type attribute.
    pub fn r#type(mut self, value: impl ToString) -> Self {
        self.r#type = Some(value.to_string());
        self
    }

    /// Set the blocking attribute.
    pub fn blocking(mut self, value: impl ToString) -> Self {
        self.blocking = Some(value.to_string());
        self
    }

    /// Build the link element.
    pub fn build(self) -> Element {
        document::Link(
            LinkProps::builder()
                .rel(self.rel)
                .media(self.media)
                .title(self.title)
                .disabled(self.disabled)
                .r#as(self.r#as)
                .sizes(self.sizes)
                .href(self.href)
                .crossorigin(self.crossorigin)
                .referrerpolicy(self.referrerpolicy)
                .fetchpriority(self.fetchpriority)
                .hreflang(self.hreflang)
                .integrity(self.integrity)
                .r#type(self.r#type)
                .blocking(self.blocking)
                .build(),
        )
    }
}

/// Create a new document link builder.
///
/// # Example
///
/// ```rust,ignore
/// doc_link()
///     .rel("preload")
///     .href("/fonts/font.woff2")
///     .r#as("font")
///     .crossorigin("anonymous")
///     .build()
/// ```
pub fn doc_link() -> DocLinkBuilder {
    DocLinkBuilder::new()
}
