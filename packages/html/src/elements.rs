#![allow(non_upper_case_globals)]

#[cfg(feature = "hot-reload-context")]
use dioxus_core_types::HotReloadingContext;
use dioxus_html_internal_macro::impl_extension_attributes;

#[cfg(feature = "hot-reload-context")]
use crate::{map_global_attributes, map_svg_attributes};

#[cfg(feature = "hot-reload-context")]
macro_rules! impl_attribute_match {
    (
        $attr:ident $(#[$leading:meta])* #[attr(name = $name:literal)] $(#[$attr_method:meta])* $fil:ident,
    ) => {
        if $attr == stringify!($fil) {
            return Some(($name, None));
        }
    };

    (
        $attr:ident $(#[$leading:meta])* #[attr(namespace = $ns:literal)] $(#[$attr_method:meta])* $fil:ident,
    ) => {
        if $attr == stringify!($fil) {
            return Some((stringify!($fil), Some($ns)));
        }
    };

    (
        $attr:ident $(#[$attr_method:meta])* $fil:ident,
    ) => {
        if $attr == stringify!($fil) {
            return Some((stringify!($fil), None));
        }
    };
}

#[cfg(feature = "html-to-rsx")]
macro_rules! impl_html_to_rsx_attribute_match {
    (
        $attr:ident $(#[$leading:meta])* #[attr(name = $name:literal)] $(#[$attr_method:meta])* $fil:ident
    ) => {
        if $attr == $name {
            return Some(stringify!($fil));
        }
    };

    (
        $attr:ident $(#[$attr_method:meta])* $fil:ident $($_:tt)*
    ) => {
        if $attr == stringify!($fil) {
            return Some(stringify!($fil));
        }
    };
}

macro_rules! impl_element {
    (
        $(#[$attr:meta])*
        $name:ident None {
            $(
                $(#[$attr_method:meta])*
                $fil:ident,
            )*
        }
    ) => {
        #[allow(non_camel_case_types)]
        $(#[$attr])*
        ///
        /// ## Usage in rsx
        ///
        /// ```rust, no_run
        /// # use dioxus::prelude::*;
        /// # let attributes = vec![];
        /// # fn ChildComponent() -> Element { unimplemented!() }
        /// # let raw_expression: Element = rsx! {};
        /// rsx! {
        ///     // Elements are followed by braces that surround any attributes and children for that element
        #[doc = concat!("    ", stringify!($name), " {")]
        ///         // Add any attributes first
        ///         class: "my-class",
        ///         "custom-attribute-name": "value",
        ///         // Then add any attributes you are spreading into this element
        ///         ..attributes,
        ///         // Then add any children elements, components, text nodes, or raw expressions
        ///         div {}
        ///         ChildComponent {}
        ///         "child text"
        ///         {raw_expression}
        ///     }
        /// };
        /// ```
        #[doc(hidden)]
        pub mod $name {
            #[allow(unused)]
            use super::*;

            pub(super) const TAG_NAME: &'static str = stringify!($name);
            pub(super) const NAME_SPACE: Option<&'static str> = None;

            #[doc(hidden)]
            pub struct Tag;

            impl dioxus_core::view::TagName for Tag {
                const NAME: &'static str = TAG_NAME;
                const NAMESPACE: Option<&'static str> = NAME_SPACE;
            }

            pub const fn new() -> dioxus_core::view::El<Tag, (), ()> {
                dioxus_core::view::el::<Tag>()
            }

        }
    };

    (
        $(#[$attr:meta])*
        $name:ident $namespace:literal {
            $(
                $(#[$attr_method:meta])*
                $fil:ident,
            )*
        }
    ) => {
        $(#[$attr])*
        ///
        /// ## Usage in rsx
        ///
        /// ```rust, no_run
        /// # use dioxus::prelude::*;
        /// # let attributes = vec![];
        /// # fn ChildComponent() -> Element { unimplemented!() }
        /// # let raw_expression: Element = rsx! {};
        /// rsx! {
        ///     // Elements are followed by braces that surround any attributes and children for that element
        #[doc = concat!("    ", stringify!($name), " {")]
        ///         // Add any attributes first
        ///         color: "red",
        ///         "custom-attribute-name": "value",
        ///         // Then add any attributes you are spreading into this element
        ///         ..attributes,
        ///         // Then add any children elements, components, text nodes, or raw expressions
        ///         circle { cx: "10", cy: "10", r: "2", fill: "red" }
        ///         ChildComponent {}
        ///         "child text"
        ///         {raw_expression}
        ///     }
        /// };
        /// ```
        #[doc(hidden)]
        pub mod $name {
            #[allow(unused)]
            use super::*;

            pub(super) const TAG_NAME: &'static str = stringify!($name);
            pub(super) const NAME_SPACE: Option<&'static str> = Some($namespace);

            #[doc(hidden)]
            pub struct Tag;

            impl dioxus_core::view::TagName for Tag {
                const NAME: &'static str = TAG_NAME;
                const NAMESPACE: Option<&'static str> = NAME_SPACE;
            }

            pub const fn new() -> dioxus_core::view::El<Tag, (), ()> {
                dioxus_core::view::el::<Tag>()
            }

        }
    };

    (
        $(#[$attr:meta])*
        $element:ident [$name:literal, $namespace:tt] {
            $(
                $(#[$attr_method:meta])*
                $fil:ident,
            )*
        }
    ) => {
        #[allow(non_camel_case_types)]
        $(#[$attr])*
        ///
        /// ## Usage in rsx
        ///
        /// ```rust, no_run
        /// # use dioxus::prelude::*;
        /// # let attributes = vec![];
        /// # fn ChildComponent() -> Element { unimplemented!() }
        /// # let raw_expression: Element = rsx! {};
        /// rsx! {
        ///     // Elements are followed by braces that surround any attributes and children for that element
        #[doc = concat!("    ", stringify!($element), " {")]
        ///         // Add any attributes first
        ///         color: "red",
        ///         "custom-attribute-name": "value",
        ///         // Then add any attributes you are spreading into this element
        ///         ..attributes,
        ///         // Then add any children elements, components, text nodes, or raw expressions
        ///         circle { cx: "10", cy: "10", r: "2", fill: "red" }
        ///         ChildComponent {}
        ///         "child text"
        ///         {raw_expression}
        ///     }
        /// };
        /// ```
        #[doc(hidden)]
        pub mod $element {
            #[allow(unused)]
            use super::*;

            pub(super) const TAG_NAME: &'static str = $name;
            pub(super) const NAME_SPACE: Option<&'static str> = Some($namespace);

            #[doc(hidden)]
            pub struct Tag;

            impl dioxus_core::view::TagName for Tag {
                const NAME: &'static str = TAG_NAME;
                const NAMESPACE: Option<&'static str> = NAME_SPACE;
            }

            pub const fn new() -> dioxus_core::view::El<Tag, (), ()> {
                dioxus_core::view::el::<Tag>()
            }

        }
    }
}

macro_rules! impl_element_constructor {
    (
        $(#[$attr:meta])*
        $name:ident None {
            $(
                $(#[$attr_method:meta])*
                $fil:ident,
            )*
        }
    ) => {
        $(#[$attr])*
        pub const fn $name() -> dioxus_core::view::El<$name::Tag, (), ()> {
            $name::new()
        }
    };

    (
        $(#[$attr:meta])*
        $name:ident "http://www.w3.org/2000/svg" {
            $(
                $(#[$attr_method:meta])*
                $fil:ident,
            )*
        }
    ) => {
        $(#[$attr])*
        pub const fn $name() -> dioxus_core::view::El<$name::Tag, (), ()> {
            $name::new()
        }
    };

    (
        $(#[$attr:meta])*
        $name:ident $namespace:literal {
            $(
                $(#[$attr_method:meta])*
                $fil:ident,
            )*
        }
    ) => {
        $(#[$attr])*
        pub const fn $name() -> dioxus_core::view::El<$name::Tag, (), ()> {
            $name::new()
        }
    };

    (
        $(#[$attr:meta])*
        $element:ident [$name:literal, "http://www.w3.org/2000/svg"] {
            $(
                $(#[$attr_method:meta])*
                $fil:ident,
            )*
        }
    ) => {
        $(#[$attr])*
        pub const fn $element() -> dioxus_core::view::El<$element::Tag, (), ()> {
            $element::new()
        }
    };

    (
        $(#[$attr:meta])*
        $element:ident [$name:literal, $namespace:tt] {
            $(
                $(#[$attr_method:meta])*
                $fil:ident,
            )*
        }
    ) => {
        $(#[$attr])*
        pub const fn $element() -> dioxus_core::view::El<$element::Tag, (), ()> {
            $element::new()
        }
    };
}

#[cfg(feature = "hot-reload-context")]
macro_rules! impl_element_match {
    (
        $el:ident $name:ident None {
            $(
                $(#[$attr_method:meta])*
                $fil:ident,
            )*
        }
    ) => {
        if $el == stringify!($name) {
            return Some((stringify!($name), None));
        }
    };

    (
        $el:ident $name:ident $namespace:literal {
            $(
                $(#[$attr_method:meta])*
                $fil:ident,
            )*
        }
    ) => {
        if $el == stringify!($name) {
            return Some((stringify!($name), Some($namespace)));
        }
    };

    (
        $el:ident $name:ident [$_:literal, $namespace:tt] {
            $(
                $(#[$attr_method:meta])*
                $fil:ident,
            )*
        }
    ) => {
        if $el == stringify!($name) {
            return Some((stringify!($name), Some($namespace)));
        }
    };
}

#[cfg(feature = "hot-reload-context")]
macro_rules! impl_element_match_attributes {
    (
        $el:ident $attr:ident $name:ident None {
            $(
                $(#[$attr_method:meta])*
                $fil:ident,
            )*
        }
    ) => {
        if $el == stringify!($name) {
            $(
                impl_attribute_match!(
                    $attr $(#[$attr_method])* $fil,
                );
            )*

            return impl_map_global_attributes!($el $attr $name None);
        }
    };

    (
        $el:ident $attr:ident $name:ident $namespace:tt {
            $(
                $(#[$attr_method:meta])*
                $fil:ident,
            )*
        }
    ) => {
        if $el == stringify!($name) {
            $(
                impl_attribute_match!(
                    $attr $(#[$attr_method])* $fil,
                );
            )*

            return impl_map_global_attributes!($el $attr $name $namespace);
        }
    }
}

#[cfg(feature = "hot-reload-context")]
macro_rules! impl_map_global_attributes {
    (
        $el:ident $attr:ident $element:ident None
    ) => {
        map_global_attributes($attr)
    };

    (
        $el:ident $attr:ident $element:ident $namespace:literal
    ) => {
        if $namespace == "http://www.w3.org/2000/svg" {
            map_svg_attributes($attr)
        } else {
            map_global_attributes($attr)
        }
    };

    (
        $el:ident $attr:ident $element:ident [$name:literal, $namespace:tt]
    ) => {
        if $namespace == "http://www.w3.org/2000/svg" {
            map_svg_attributes($attr)
        } else {
            map_global_attributes($attr)
        }
    };
}

macro_rules! impl_attribute_group_extension {
    ($name:ident "http://www.w3.org/2000/svg") => {
        impl<Attrs, Children> crate::attribute_groups::SvgAttributesExtension
            for dioxus_core::view::El<$name::Tag, Attrs, Children>
        {
        }
    };

    ($name:ident [$_tag:literal, "http://www.w3.org/2000/svg"]) => {
        impl<Attrs, Children> crate::attribute_groups::SvgAttributesExtension
            for dioxus_core::view::El<$name::Tag, Attrs, Children>
        {
        }
    };

    ($name:ident $namespace:tt) => {
        impl<Attrs, Children> crate::attribute_groups::GlobalAttributesExtension
            for dioxus_core::view::El<$name::Tag, Attrs, Children>
        {
        }
    };
}

macro_rules! builder_constructors {
    (
        $(
            $(#[$attr:meta])*
            $name:ident $namespace:tt {
                $(
                    $(#[$attr_method:meta])*
                    $fil:ident,
                )*
            };
         )*
        ) => {
        #[cfg(feature = "hot-reload-context")]
        pub struct HtmlCtx;

        #[cfg(feature = "hot-reload-context")]
        impl HotReloadingContext for HtmlCtx {
            fn map_attribute(element: &str, attribute: &str) -> Option<(&'static str, Option<&'static str>)> {
                $(
                    impl_element_match_attributes!(
                        element attribute $name $namespace {
                            $(
                                $(#[$attr_method])*
                                $fil,
                            )*
                        }
                    );
                )*
                None
            }

            fn map_element(element: &str) -> Option<(&'static str, Option<&'static str>)> {
                $(
                    impl_element_match!(
                        element $name $namespace {
                            $(
                                $(#[$attr_method])*
                                $fil,
                            )*
                        }
                    );
                )*
                None
            }
        }

        #[cfg(feature = "html-to-rsx")]
        pub fn map_html_attribute_to_rsx(html: &str) -> Option<&'static str> {
            $(
                $(
                    impl_html_to_rsx_attribute_match!(
                        html $(#[$attr_method])* $fil
                    );
                )*
            )*

            if let Some(name) = crate::map_html_global_attributes_to_rsx(html) {
                return Some(name);
            }

            if let Some(name) = crate::map_html_svg_attributes_to_rsx(html) {
                return Some(name);
            }

            None
        }

        #[cfg(feature = "html-to-rsx")]
        pub fn map_html_element_to_rsx(html: &str) -> Option<&'static str> {
            $(
                if html == stringify!($name) {
                    return Some(stringify!($name));
                }
            )*

            None
        }

        $(
            impl_element!(
                $(#[$attr])*
                $name $namespace {
                    $(
                        $(#[$attr_method])*
                        $fil,
                    )*
                }
            );
        )*

        $(
            impl_element_constructor!(
                $(#[$attr])*
                $name $namespace {
                    $(
                        $(#[$attr_method])*
                        $fil,
                    )*
                }
            );
            impl_attribute_group_extension!($name $namespace);
        )*

        pub(crate) mod extensions {
            use super::*;
            $(
                impl_extension_attributes![$name { $($(#[$attr_method])* $fil ,)* } for_el];
            )*
        }
    };
}

// Organized in the same order as
// https://developer.mozilla.org/en-US/docs/Web/HTML/Element
//
// Does not include obsolete elements.
//
// This namespace represents a collection of modern HTML-5 compatible elements.
//
// This list does not include obsolete, deprecated, experimental, or poorly supported elements.
builder_constructors! {
    // Document metadata

    /// Build a
    /// [`<base>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/base)
    /// element.
    ///
    base None {
        href,
        target,
    };

    /// Build a
    /// [`<head>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/head)
    /// element.
    head None {};

    /// Build a
    /// [`<link>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link)
    /// element.
    link None {
        // as: Mime,
        crossorigin,
        href,
        hreflang,
        media, // FIXME media query
        rel,
        sizes, // FIXME
        title, // FIXME
        #[attr(name = "type")]
        r#type,
        integrity,
        disabled,
        referrerpolicy,
        fetchpriority,
        blocking,
        #[attr(name = "as")]
        r#as,
    };

    /// Build a
    /// [`<meta>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta)
    /// element.
    meta None {
        charset, // FIXME IANA standard names
        content,
        #[attr(name = "http-equiv")]
        http_equiv,
        name,
        property,
    };

    /// Build a
    /// [`<style>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/style)
    /// element.
    style None {
        #[attr(name = "type")]
        r#type,
        media, // FIXME media query
        nonce,
        title, // FIXME
    };

    /// Build a
    /// [`<title>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/title)
    /// element.
    title None { };

    // Sectioning root

    /// Build a
    /// [`<body>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/body)
    /// element.
    body None {};

    // ------------------
    // Content sectioning
    // ------------------

    /// Build a
    /// [`<address>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/address)
    /// element.
    address None {};

    /// Build a
    /// [`<article>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/article)
    /// element.
    article None {};

    /// Build a
    /// [`<aside>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/aside)
    /// element.
    aside None {};

    /// Build a
    /// [`<footer>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/footer)
    /// element.
    footer None {};

    /// Build a
    /// [`<header>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/header)
    /// element.
    header None {};

    /// Build a
    /// [`<hgroup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/hgroup)
    /// element.
    hgroup None {};

    /// Build a
    /// [`<h1>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h1)
    /// element.
    ///
    /// # About
    /// - The HTML `<h1>` element is found within the `<body>` tag.
    /// - Headings can range from `<h1>` to `<h6>`.
    /// - The most important heading is `<h1>` and the least important heading is `<h6>`.
    /// - The `<h1>` heading is the first heading in the document.
    /// - The `<h1>` heading is usually a large bolded font.
    h1 None {};

    /// Build a
    /// [`<h2>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h2)
    /// element.
    ///
    /// # About
    /// - The HTML `<h2>` element is found within the `<body>` tag.
    /// - Headings can range from `<h1>` to `<h6>`.
    /// - The most important heading is `<h1>` and the least important heading is `<h6>`.
    /// - The `<h2>` heading is the second heading in the document.
    /// - The `<h2>` heading is usually a large bolded font.
    h2 None {};

    /// Build a
    /// [`<h3>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h3)
    /// element.
    ///
    /// # About
    /// - The HTML `<h1>` element is found within the `<body>` tag.
    /// - Headings can range from `<h1>` to `<h6>`.
    /// - The most important heading is `<h1>` and the least important heading is `<h6>`.
    /// - The `<h1>` heading is the first heading in the document.
    /// - The `<h1>` heading is usually a large bolded font.
    h3 None {};

    /// Build a
    /// [`<h4>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h4)
    /// element.
    h4 None {};

    /// Build a
    /// [`<h5>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h5)
    /// element.
    h5 None {};

    /// Build a
    /// [`<h6>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h6)
    /// element.
    h6 None {};

    /// Build a
    /// [`<main>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/main)
    /// element.
    main None {};

    /// Build a
    /// [`<nav>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/nav)
    /// element.
    nav None {};

    /// Build a
    /// [`<section>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/section)
    /// element.
    section None {};

    // Text content

    /// Build a
    /// [`<blockquote>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/blockquote)
    /// element.
    blockquote None {
        cite,
    };
    /// Build a
    /// [`<dd>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dd)
    /// element.
    dd None {};

    /// Build a
    /// [`<div>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/div)
    /// element.
    ///
    /// Part of the HTML namespace. Only works in HTML-compatible renderers
    ///
    /// ## Definition and Usage
    /// - The `<div>` tag defines a division or a section in an HTML document.
    /// - The `<div>` tag is used as a container for HTML elements - which is then styled with CSS or manipulated with  JavaScript.
    /// - The `<div>` tag is easily styled by using the class or id attribute.
    /// - Any sort of content can be put inside the `<div>` tag!
    ///
    /// Note: By default, browsers always place a line break before and after the `<div>` element.
    ///
    /// ## References:
    /// - <https://developer.mozilla.org/en-US/docs/Web/HTML/Element/div>
    /// - <https://www.w3schools.com/tags/tag_div.asp>
    div None {};

    /// Build a
    /// [`<dl>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dl)
    /// element.
    dl None {};

    /// Build a
    /// [`<dt>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dt)
    /// element.
    dt None {};

    /// Build a
    /// [`<figcaption>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/figcaption)
    /// element.
    figcaption None {};

    /// Build a
    /// [`<figure>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/figure)
    /// element.
    figure None {};

    /// Build a
    /// [`<hr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/hr)
    /// element.
    hr None {};

    /// Build a
    /// [`<li>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/li)
    /// element.
    li None {
        value,
    };

    /// Build a
    /// [`<ol>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ol)
    /// element.
    ol None {
        reversed,
        start,
        #[attr(name = "type")]
        r#type,
    };

    /// Build a
    /// [`<p>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/p)
    /// element.
    p None {};

    /// Build a
    /// [`<pre>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/pre)
    /// element.
    pre None {};

    /// Build a
    /// [`<ul>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ul)
    /// element.
    ul None {};


    // Inline text semantics

    /// Build a
    /// [`<a>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/a)
    /// element.
    a None {
        download,
        href,
        hreflang,
        target,
        #[attr(name = "type")]
        r#type,
        // ping: SpacedList<Uri>,
        // rel: SpacedList<LinkType>,
        ping,
        rel,
    };

    /// Build a
    /// [`<abbr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/abbr)
    /// element.
    abbr None {};

    /// Build a
    /// [`<b>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/b)
    /// element.
    b None {};

    /// Build a
    /// [`<bdi>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/bdi)
    /// element.
    bdi None {};

    /// Build a
    /// [`<bdo>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/bdo)
    /// element.
    bdo None {};

    /// Build a
    /// [`<br>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/br)
    /// element.
    br None {};

    /// Build a
    /// [`<cite>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/cite)
    /// element.
    cite None {};

    /// Build a
    /// [`<code>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/code)
    /// element.
    code None {
        language,
    };

    /// Build a
    /// [`<data>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/data)
    /// element.
    data None {
        value,
    };

    /// Build a
    /// [`<dfn>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dfn)
    /// element.
    dfn None {};

    /// Build a
    /// [`<em>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/em)
    /// element.
    em None {};

    /// Build a
    /// [`<i>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/i)
    /// element.
    i None {};

    /// Build a
    /// [`<kbd>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/kbd)
    /// element.
    kbd None {};

    /// Build a
    /// [`<mark>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/mark)
    /// element.
    mark None {};

    /// Build a
    /// [`<menu>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/menu)
    /// element.
    menu None {};

    /// Build a
    /// [`<q>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/q)
    /// element.
    q None {
        cite,
    };


    /// Build a
    /// [`<rp>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/rp)
    /// element.
    rp None {};


    /// Build a
    /// [`<rt>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/rt)
    /// element.
    rt None {};


    /// Build a
    /// [`<ruby>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ruby)
    /// element.
    ruby None {};

    /// Build a
    /// [`<s>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/s)
    /// element.
    s None {};

    /// Build a
    /// [`<samp>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/samp)
    /// element.
    samp None {};

    /// Build a
    /// [`<small>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/small)
    /// element.
    small None {};

    /// Build a
    /// [`<span>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/span)
    /// element.
    span None {};

    /// Build a
    /// [`<strong>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/strong)
    /// element.
    strong None {};

    /// Build a
    /// [`<sub>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/sub)
    /// element.
    sub None {};

    /// Build a
    /// [`<sup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/sup)
    /// element.
    sup None {};

    /// Build a
    /// [`<time>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/time)
    /// element.
    time None {
        datetime,
    };

    /// Build a
    /// [`<u>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/u)
    /// element.
    u None {};

    /// Build a
    /// [`<var>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/var)
    /// element.
    var None {};

    /// Build a
    /// [`<wbr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/wbr)
    /// element.
    wbr None {};


    // Image and multimedia

    /// Build a
    /// [`<area>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/area)
    /// element.
    area None {
        alt,
        coords, // TODO could perhaps be validated
        download,
        href,
        hreflang,
        shape,
        target,
        // ping: SpacedList<Uri>,
        // rel: SpacedSet<LinkType>,
    };

    /// Build a
    /// [`<audio>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/audio)
    /// element.
    audio None {
        autoplay,
        controls,
        crossorigin,
        muted,
        preload,
        src,
        #[attr(name = "loop")]
        r#loop,
    };

    /// Build a
    /// [`<img>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/img)
    /// element.
    img None {
        alt,
        crossorigin,
        decoding,
        height,
        ismap,
        loading,
        src,
        srcset, // FIXME this is much more complicated
        usemap, // FIXME should be a fragment starting with '#'
        width,
        referrerpolicy,
        sizes, // FIXME
        elementtiming,
        fetchpriority,
        attributionsrc,
    };

    /// Build a
    /// [`<map>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/map)
    /// element.
    map None {
        name,
    };

    /// Build a
    /// [`<track>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/track)
    /// element.
    track None {
        default,
        kind,
        label,
        src,
        srclang,
    };

    /// Build a
    /// [`<video>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/video)
    /// element.
    video None {
        autoplay,
        controls,
        crossorigin,
        height,
        #[attr(name = "loop")]
        r#loop,
        muted,
        preload,
        playsinline,
        poster,
        src,
        width,
    };


    // Embedded content

    /// Build a
    /// [`<embed>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/embed)
    /// element.
    embed None {
        height,
        src,
        #[attr(name = "type")]
        r#type,
        width,
    };

    /// Build a
    /// [`<iframe>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/iframe)
    /// element.
    iframe None {
        allow,
        allowfullscreen,
        allowpaymentrequest,
        height,
        name,
        referrerpolicy,
        src,
        srcdoc,
        width,

        #[attr(name = "marginWidth")]

        margin_width,
        align,
        longdesc,

        scrolling,
        #[attr(name = "marginHeight")]
        margin_height,
        #[attr(name = "frameBorder")]
        frame_border,
        // sandbox: SpacedSet<Sandbox>,
    };

    /// Build a
    /// [`<object>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/object)
    /// element.
    object None {
        data,
        form,
        height,
        name,
        #[attr(name = "type")]
        r#type,
        typemustmatch,
        usemap, // TODO should be a fragment starting with '#'
        width,
    };

    /// Build a
    /// [`<param>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/param)
    /// element.
    param None {
        name,
        value,
    };

    /// Build a
    /// [`<picture>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/picture)
    /// element.
    picture None {};

    /// Build a
    /// [`<source>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/source)
    /// element.
    source None {
        src,
        #[attr(name = "type")]
        r#type,
        srcset,
        media,
        sizes,
        width,
        height,
    };


    // Scripting

    /// Build a
    /// [`<canvas>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/canvas)
    /// element.
    canvas None {
        height,
        width,
    };

    /// Build a
    /// [`<noscript>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/noscript)
    /// element.
    noscript None {};

    /// Build a
    /// [`<script>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script)
    /// element.
    ///
    /// The script HTML element is used to embed executable code or data; this is typically used to embed or refer to
    /// JavaScript code. The script element can also be used with other languages, such as WebGL's GLSL shader
    /// programming language and JSON.
    script None {
        /// Normal script elements pass minimal information to the window.onerror for scripts which do not pass the
        /// standard CORS checks. To allow error logging for sites which use a separate domain for static media, use
        /// this attribute. See CORS settings attributes for a more descriptive explanation of its valid arguments.
        crossorigin,

        /// This Boolean attribute is set to indicate to a browser that the script is meant to be executed after the
        /// document has been parsed, but before firing DOMContentLoaded.
        ///
        /// Scripts with the defer attribute will prevent the DOMContentLoaded event from firing until the script has
        /// loaded and finished evaluating.
        ///
        /// ----
        /// ### Warning:
        ///
        /// This attribute must not be used if the src attribute is absent (i.e. for inline scripts), in this
        /// case it would have no effect.
        ///
        /// ----
        ///
        /// The defer attribute has no effect on module scripts — they defer by default.
        /// Scripts with the defer attribute will execute in the order in which they appear in the document.
        ///
        /// This attribute allows the elimination of parser-blocking JavaScript where the browser would have to load and
        /// evaluate scripts before continuing to parse. async has a similar effect in this case.
        defer,
        integrity,
        nomodule,
        nonce,
        src,
        text,
        fetchpriority,
        referrerpolicy,

        #[attr(name = "async")]

        r#async,
        #[attr(name = "type")]
        r#type, // TODO could be an enum
        #[attr(name = "script")]
        r#script,
    };


    // Demarcating edits

    /// Build a
    /// [`<del>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/del)
    /// element.
    del None {
        cite,
        datetime,
    };

    /// Build a
    /// [`<ins>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ins)
    /// element.
    ins None {
        cite,
        datetime,
    };


    // Table content

    /// Build a
    /// [`<caption>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/caption)
    /// element.
    caption None {};

    /// Build a
    /// [`<col>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/col)
    /// element.
    col None {
        span,
    };

    /// Build a
    /// [`<colgroup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/colgroup)
    /// element.
    colgroup None {
        span,
    };

    /// Build a
    /// [`<table>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/table)
    /// element.
    table None {};

    /// Build a
    /// [`<tbody>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tbody)
    /// element.
    tbody None {};

    /// Build a
    /// [`<td>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/td)
    /// element.
    td None {
        colspan,
        rowspan,
        // headers: SpacedSet<Id>,
    };

    /// Build a
    /// [`<tfoot>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tfoot)
    /// element.
    tfoot None {};

    /// Build a
    /// [`<th>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/th)
    /// element.
    th None {
        abbr,
        colspan,
        rowspan,
        scope,
        // headers: SpacedSet<Id>,
    };

    /// Build a
    /// [`<thead>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/thead)
    /// element.
    thead None {};

    /// Build a
    /// [`<tr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tr)
    /// element.
    tr None {};


    // Forms

    /// Build a
    /// [`<button>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/button)
    /// element.
    button None {
        autofocus,
        disabled,
        form,
        formaction,
        formenctype,
        formmethod,
        formnovalidate,
        formtarget,
        name,
        popovertarget,
        popovertargetaction,
        value,
        #[attr(name = "type")]
        r#type,
    };

    /// Build a
    /// [`<datalist>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/datalist)
    /// element.
    datalist None {};

    /// Build a
    /// [`<fieldset>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/fieldset)
    /// element.
    fieldset None {
        disabled,
        form,
        name,
    };

    /// Build a
    /// [`<form>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
    /// element.
    form None {
        // accept-charset: SpacedList<CharacterEncoding>,
        action,
        autocomplete,
        enctype,
        method,
        name,
        novalidate,
        target,
    };

    /// Build a
    /// [`<input>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input)
    /// element.
    input None {
        accept,
        alt,
        autocomplete,
        /// cf. <https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Global_attributes/autocorrect>
        autocorrect,
        autofocus,
        capture,
        checked,
        #[attr(name = "webkitdirectory")]
        directory,
        disabled,
        form,
        formaction,
        formenctype,
        formmethod,
        formnovalidate,
        formtarget,
        height,
        initial_checked,
        list,
        max,
        maxlength,
        min,
        minlength,
        multiple,
        name,
        pattern,
        popovertarget,
        popovertargetaction,
        placeholder,
        readonly,
        required,
        size,
        spellcheck,
        src,
        step,
        tabindex,
        width,

        /// The type of input
        ///
        /// Here are the different input types you can use in HTML:
        ///
        /// - `button`
        /// - `checkbox`
        /// - `color`
        /// - `date`
        /// - `datetime-local`
        /// - `email`
        /// - `file`
        /// - `hidden`
        /// - `image`
        /// - `month`
        /// - `number`
        /// - `password`
        /// - `radio`
        /// - `range`
        /// - `reset`
        /// - `search`
        /// - `submit`
        /// - `tel`
        /// - `text`
        /// - `time`
        /// - `url`
        /// - `week`

        #[attr(name = "type")]

        r#type,
        // value: String,
        #[attr(volatile)]
        value,
        initial_value,
    };

    /// Build a
    /// [`<label>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/label)
    /// element.
    label None {
        form,
        #[attr(name = "for")]
        r#for,
    };

    /// Build a
    /// [`<legend>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/legend)
    /// element.
    legend None {};

    /// Build a
    /// [`<meter>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meter)
    /// element.
    meter None {
        value,
        min,
        max,
        low,
        high,
        optimum,
        form,
    };

    /// Build a
    /// [`<optgroup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/optgroup)
    /// element.
    optgroup None {
        disabled,
        label,
    };

    /// Build a
    /// [`<option>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/option)
    /// element.
    option None {
        disabled,
        label,


        value,

        #[attr(volatile)]
        selected,
        initial_selected,
    };

    /// Build a
    /// [`<output>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/output)
    /// element.
    output None {
        form,
        name,
        // r#for: SpacedSet<Id>,
    };

    /// Build a
    /// [`<progress>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/progress)
    /// element.
    progress None {
        max,
        value,
    };

    /// Build a
    /// [`<select>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/select)
    /// element.
    select None {
        // defined below
        // value: String,
        autocomplete,
        autofocus,
        disabled,
        form,
        multiple,
        name,
        required,
        size,
        #[attr(volatile)]
        value,
    };

    /// Build a
    /// [`<textarea>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/textarea)
    /// element.
    textarea None {
        autocomplete,
        /// cf. <https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Global_attributes/autocorrect>
        autocorrect,
        autofocus,
        cols,
        disabled,
        form,
        maxlength,
        minlength,
        name,
        placeholder,
        readonly,
        required,
        rows,
        spellcheck,
        wrap,
        #[attr(volatile)]
        value,

        initial_value,
    };


    // Interactive elements

    /// Build a
    /// [`<details>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/details)
    /// element.
    details None {
        open,
    };

    /// Build dialog
    /// [`<dialog>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dialog)
    /// element.
    dialog None {
        open,
    };

    /// Build a
    /// [`<summary>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/summary)
    /// element.
    summary None {};

    // Web components

    /// Build a
    /// [`<slot>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/slot)
    /// element.
    slot None {
        name,
    };

    /// Build a
    /// [`<template>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/template)
    /// element.
    template None {};

    // SVG components
    /// Build a
    /// [`<svg>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/svg)
    /// element.
    svg "http://www.w3.org/2000/svg" { };


    // /// Build a
    // /// [`<a>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/a)
    // /// element.
    // a "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<animate>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/animate)
    /// element.
    animate "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<animateMotion>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/animateMotion)
    /// element.
    animateMotion "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<animateTransform>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/animateTransform)
    /// element.
    animateTransform "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<circle>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/circle)
    /// element.
    circle "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<clipPath>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/clipPath)
    /// element.
    clipPath "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<defs>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/defs)
    /// element.
    defs "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<desc>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/desc)
    /// element.
    desc "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<discard>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/discard)
    /// element.
    discard "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<ellipse>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/ellipse)
    /// element.
    ellipse "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feBlend>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feBlend)
    /// element.
    feBlend "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feColorMatrix>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feColorMatrix)
    /// element.
    feColorMatrix "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feComponentTransfer>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feComponentTransfer)
    /// element.
    feComponentTransfer "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feComposite>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feComposite)
    /// element.
    feComposite "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feConvolveMatrix>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feConvolveMatrix)
    /// element.
    feConvolveMatrix "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feDiffuseLighting>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feDiffuseLighting)
    /// element.
    feDiffuseLighting "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feDisplacementMap>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feDisplacementMap)
    /// element.
    feDisplacementMap "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feDistantLight>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feDistantLight)
    /// element.
    feDistantLight "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feDropShadow>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feDropShadow)
    /// element.
    feDropShadow "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feFlood>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feFlood)
    /// element.
    feFlood "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feFuncA>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feFuncA)
    /// element.
    feFuncA "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feFuncB>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feFuncB)
    /// element.
    feFuncB "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feFuncG>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feFuncG)
    /// element.
    feFuncG "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feFuncR>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feFuncR)
    /// element.
    feFuncR "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feGaussianBlur>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feGaussianBlur)
    /// element.
    feGaussianBlur "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feImage>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feImage)
    /// element.
    feImage "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feMerge>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feMerge)
    /// element.
    feMerge "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feMergeNode>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feMergeNode)
    /// element.
    feMergeNode "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feMorphology>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feMorphology)
    /// element.
    feMorphology "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feOffset>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feOffset)
    /// element.
    feOffset "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<fePointLight>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/fePointLight)
    /// element.
    fePointLight "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feSpecularLighting>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feSpecularLighting)
    /// element.
    feSpecularLighting "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feSpotLight>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feSpotLight)
    /// element.
    feSpotLight "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feTile>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feTile)
    /// element.
    feTile "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feTurbulence>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feTurbulence)
    /// element.
    feTurbulence "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<filter>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/filter)
    /// element.
    filter "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<foreignObject>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/foreignObject)
    /// element.
    foreignObject "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<g>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/g)
    /// element.
    g "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<hatch>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/hatch)
    /// element.
    hatch "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<hatchpath>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/hatchpath)
    /// element.
    hatchpath "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<image>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/image)
    /// element.
    image "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<line>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/line)
    /// element.
    line "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<linearGradient>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/linearGradient)
    /// element.
    linearGradient "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<marker>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/marker)
    /// element.
    marker "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<mask>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/mask)
    /// element.
    mask "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<metadata>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/metadata)
    /// element.
    metadata "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<mpath>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/mpath)
    /// element.
    mpath "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<path>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/path)
    /// element.
    path "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<pattern>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/pattern)
    /// element.
    pattern "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<polygon>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/polygon)
    /// element.
    polygon "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<polyline>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/polyline)
    /// element.
    polyline "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<radialGradient>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/radialGradient)
    /// element.
    radialGradient "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<rect>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/rect)
    /// element.
    rect "http://www.w3.org/2000/svg" {};

    // /// Build a
    // /// [`<script>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/script)
    // /// element.
    // script "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<set>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/set)
    /// element.
    set "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<stop>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/stop)
    /// element.
    stop "http://www.w3.org/2000/svg" {};

    // /// Build a
    // /// [`<style>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/style)
    // /// element.
    // style "http://www.w3.org/2000/svg" {};

    // /// Build a
    // /// [`<svg>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/svg)
    // /// element.
    // svg "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<switch>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/switch)
    /// element.
    switch "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<symbol>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/symbol)
    /// element.
    symbol "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<text>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/text)
    /// element.
    text "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<textPath>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/textPath)
    /// element.
    textPath "http://www.w3.org/2000/svg" {};

    // /// Build a
    // /// [`<title>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/title)
    // /// element.
    // title "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<tspan>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/tspan)
    /// element.
    tspan "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<view>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/view)
    /// element.
    view "http://www.w3.org/2000/svg" {};

    // /// Build a
    // /// [`<use>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/use)
    // /// element.
    r#use ["use", "http://www.w3.org/2000/svg"] {
        href,
    };

    // MathML elements

    /// Build a
    /// [`<annotation>`](https://w3c.github.io/mathml-core/#dfn-annotation)
    /// element.
    annotation "http://www.w3.org/1998/Math/MathML" {
            encoding,
    };

    /// Build a
    /// [`<annotation-xml>`](https://w3c.github.io/mathml-core/#dfn-annotation-xml)
    /// element.
    annotationXml ["annotation-xml", "http://www.w3.org/1998/Math/MathML"] {
            encoding,
    };

    /// Build a
    /// [`<merror>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/merror)
    /// element.
    merror "http://www.w3.org/1998/Math/MathML" {};

    /// Build a
    /// [`<math>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/math)
    /// element.
    math "http://www.w3.org/1998/Math/MathML" {
        display,
    };

    /// Build a
    /// [`<mfrac>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mfrac)
    /// element.
    mfrac "http://www.w3.org/1998/Math/MathML" {
        linethickness,
    };

    /// Build a
    /// [`<mi>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mi)
    /// element.
    mi "http://www.w3.org/1998/Math/MathML" {
        mathvariant,
    };

    /// Build a
    /// [`<mmultiscripts>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mmultiscripts)
    /// element.
    mmultiscripts "http://www.w3.org/1998/math/mathml" {};

    /// Build a
    /// [`<mn>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mn)
    /// element.
    mn "http://www.w3.org/1998/Math/MathML" {};

    /// Build a
    /// [`<mo>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mo)
    /// element.
    mo "http://www.w3.org/1998/Math/MathML" {
        fence,
        largeop,
        lspace,
        maxsize,
        minsize,
        movablelimits,
        rspace,
        separator,
        stretchy,
        symmetric,
    };

    /// Build a
    /// [`<mover>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mover)
    /// element.
    mover "http://www.w3.org/1998/Math/MathML" {
        accent,
    };

    /// Build a
    /// [`<mpadded>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mpadded)
    /// element.
    mpadded "http://www.w3.org/1998/Math/MathML" {
        depth,
        height,
        lspace,
        voffset,
        width,
    };

    /// Build a
    /// [`<mphantom>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mphantom)
    /// element.
    mphantom "http://www.w3.org/1998/Math/MathML" {};

    /// Build a
    /// [`<mprescripts>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mprescripts)
    /// element.
    mprescripts "http://www.w3.org/1998/Math/MathML" {};

    /// Build a
    /// [`<mroot>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mroot)
    /// element.
    mroot "http://www.w3.org/1998/Math/MathML" {};

    /// Build a
    /// [`<mrow>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mrow)
    /// element.
    mrow "http://www.w3.org/1998/Math/MathML" {

    };

    /// Build a
    /// [`<ms>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/ms)
    /// element.
    ms "http://www.w3.org/1998/Math/MathML" {
        lquote,
        rquote,
    };

    /// Build a
    /// [`<mspace>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mspace)
    /// element.
    mspace "http://www.w3.org/1998/Math/MathML" {
        depth,
        height,
        width,
    };

    /// Build a
    /// [`<msqrt>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/msqrt)
    /// element.
    msqrt "http://www.w3.org/1998/Math/MathML" {};

    /// Build a
    /// [`<mstyle>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mstyle)
    /// element.
    mstyle "http://www.w3.org/1998/Math/MathML" {};

    /// Build a
    /// [`<msub>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/msub)
    /// element.
    msub "http://www.w3.org/1998/Math/MathML" {};

    /// Build a
    /// [`<msubsup>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/msubsup)
    /// element.
    msubsup "http://www.w3.org/1998/Math/MathML" {};

    /// Build a
    /// [`<msup>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/msup)
    /// element.
    msup "http://www.w3.org/1998/Math/MathML" {};

    /// Build a
    /// [`<mtable>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mtable)
    /// element.
    mtable "http://www.w3.org/1998/Math/MathML" {};

    /// Build a
    /// [`<mtd>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mtd)
    /// element.
    mtd "http://www.w3.org/1998/Math/MathML" {
        columnspan,
        rowspan,
    };

    /// Build a
    /// [`<mtext>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mtext)
    /// element.
    mtext "http://www.w3.org/1998/Math/MathML" {};

    /// Build a
    /// [`<mtr>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mtr)
    /// element.
    mtr "http://www.w3.org/1998/Math/MathML" {};

    /// Build a
    /// [`<munder>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/munder)
    /// element.
    munder "http://www.w3.org/1998/Math/MathML" {
        accentunder,
    };

    /// Build a
    /// [`<munderover>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/munderover)
    /// element.
    munderover "http://www.w3.org/1998/Math/MathML" {
        accent,
        accentunder,
    };

    /// Build a
    /// [`<semantics>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/semantics)
    /// element.
    semantics "http://www.w3.org/1998/Math/MathML" {
        encoding,
    };
}
