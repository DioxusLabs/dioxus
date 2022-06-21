// map the rsx name of the element to the html name of the element and the namespace that contains it
pub fn element_to_static_str(element: &str) -> Option<(&'static str, Option<&'static str>)> {
    ELEMENTS_WITH_MAPPED_ATTRIBUTES
        .iter()
        .find(|(el, _)| *el == element)
        .map(|(el, _)| (*el, None))
        .or_else(|| {
            ELEMENTS_WITH_NAMESPACE
                .iter()
                .find(|(el, _, _)| *el == element)
                .map(|(el, ns, _)| (*el, Some(*ns)))
        })
        .or_else(|| {
            ELEMENTS_WITHOUT_NAMESPACE
                .iter()
                .find(|(el, _)| *el == element)
                .map(|(el, _)| (*el, None))
        })
}

macro_rules! builder_constructors {
    (
        $(
            $(#[$attr:meta])*
            $name:ident {
                $(
                    $(#[$attr_method:meta])*
                    $fil:ident: $vil:ident,
                )*
            };
         )*
    ) => {
        pub const ELEMENTS_WITHOUT_NAMESPACE: &'static [(&'static str, &'static [&'static str])] = &[
            $(
                (
                    stringify!($name),
                    &[
                        $(
                           stringify!($fil),
                        )*
                    ]
                ),
            )*
            ];
        };

    ( $(
        $(#[$attr:meta])*
        $name:ident <> $namespace:tt {
            $($fil:ident: $vil:ident,)*
        };
    )* ) => {
        pub const ELEMENTS_WITH_NAMESPACE: &'static [(&'static str, &'static str, &'static [&'static str])] = &[
            $(
                (
                    stringify!($name),
                    stringify!($namespace),
                    &[
                        $(
                            stringify!($fil),
                        )*
                    ]
                ),
            )*
        ];
    };
}
pub const ELEMENTS_WITH_MAPPED_ATTRIBUTES: &[(&str, &[(&str, &str)])] = &[
    ("script", &[("r#type", "type"), ("r#script", "script")]),
    ("button", &[("r#type", "type")]),
    ("select", &[("value", "value")]),
    ("option", &[("selected", "selected")]),
    ("textarea", &[("value", "value")]),
    ("label", &[("r#for", "for")]),
    ("input", &[("r#type", "type"), ("value", "value")]),
];

// Organized in the same order as
// https://developer.mozilla.org/en-US/docs/Web/HTML/Element
//
// Does not include obsolete elements.
//
// This namespace represents a collection of modern HTML-5 compatiable elements.
//
// This list does not include obsolete, deprecated, experimental, or poorly supported elements.
builder_constructors! {
    // Document metadata

    /// Build a
    /// [`<base>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/base)
    /// element.
    ///
    base {
        href: Uri,
        target: Target,
    };

    /// Build a
    /// [`<head>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/head)
    /// element.
    head {};

    /// Build a
    /// [`<link>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link)
    /// element.
    link {
        // as: Mime,
        crossorigin: CrossOrigin,
        href: Uri,
        hreflang: LanguageTag,
        media: String, // FIXME media query
        rel: LinkType,
        sizes: String, // FIXME
        title: String, // FIXME
        r#type: Mime,
        integrity: String,
    };

    /// Build a
    /// [`<meta>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta)
    /// element.
    meta {
        charset: String, // FIXME IANA standard names
        content: String,
        http_equiv: HTTPEquiv,
        name: Metadata,
    };

    /// Build a
    /// [`<style>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/style)
    /// element.
    style {
        r#type: Mime,
        media: String, // FIXME media query
        nonce: Nonce,
        title: String, // FIXME
    };

    /// Build a
    /// [`<title>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/title)
    /// element.
    title { };

    // Sectioning root

    /// Build a
    /// [`<body>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/body)
    /// element.
    body {};

    // ------------------
    // Content sectioning
    // ------------------

    /// Build a
    /// [`<address>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/address)
    /// element.
    address {};

    /// Build a
    /// [`<article>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/article)
    /// element.
    article {};

    /// Build a
    /// [`<aside>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/aside)
    /// element.
    aside {};

    /// Build a
    /// [`<footer>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/footer)
    /// element.
    footer {};

    /// Build a
    /// [`<header>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/header)
    /// element.
    header {};

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
    ///
    /// # Usage
    ///
    /// ```
    /// html!(<h1> A header element </h1>)
    /// rsx!(h1 { "A header element" })
    /// LazyNodes::new(|f| f.el(h1).children([f.text("A header element")]).finish())
    /// ```
    h1 {};


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
    ///
    /// # Usage
    /// ```
    /// html!(<h2> A header element </h2>)
    /// rsx!(h2 { "A header element" })
    /// LazyNodes::new(|f| f.el(h2).children([f.text("A header element")]).finish())
    /// ```
    h2 {};


    /// Build a
    /// [`<h3>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h3)
    /// element.
    ///
    /// # About
    /// - The HTML <h1> element is found within the <body> tag.
    /// - Headings can range from <h1> to <h6>.
    /// - The most important heading is <h1> and the least important heading is <h6>.
    /// - The <h1> heading is the first heading in the document.
    /// - The <h1> heading is usually a large bolded font.
    h3 {};
    /// Build a
    /// [`<h4>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h4)
    /// element.
    h4 {};
    /// Build a
    /// [`<h5>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h5)
    /// element.
    h5 {};
    /// Build a
    /// [`<h6>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h6)
    /// element.
    h6 {};

    /// Build a
    /// [`<main>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/main)
    /// element.
    main {};
    /// Build a
    /// [`<nav>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/nav)
    /// element.
    nav {};
    /// Build a
    /// [`<section>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/section)
    /// element.
    section {};

    // Text content

    /// Build a
    /// [`<blockquote>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/blockquote)
    /// element.
    blockquote {
        cite: Uri,
    };
    /// Build a
    /// [`<dd>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dd)
    /// element.
    dd {};

    /// Build a
    /// [`<div>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/div)
    /// element.
    ///
    /// Part of the HTML namespace. Only works in HTML-compatible renderers
    ///
    /// ## Definition and Usage
    /// - The <div> tag defines a division or a section in an HTML document.
    /// - The <div> tag is used as a container for HTML elements - which is then styled with CSS or manipulated with  JavaScript.
    /// - The <div> tag is easily styled by using the class or id attribute.
    /// - Any sort of content can be put inside the <div> tag!
    ///
    /// Note: By default, browsers always place a line break before and after the <div> element.
    ///
    /// ## Usage
    /// ```
    /// html!(<div> A header element </div>)
    /// rsx!(div { "A header element" })
    /// LazyNodes::new(|f| f.element(div, &[], &[], &[], None))
    /// ```
    ///
    /// ## References:
    /// - <https://developer.mozilla.org/en-US/docs/Web/HTML/Element/div>
    /// - <https://www.w3schools.com/tags/tag_div.asp>
    div {};

    /// Build a
    /// [`<dl>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dl)
    /// element.
    dl {};

    /// Build a
    /// [`<dt>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dt)
    /// element.
    dt {};

    /// Build a
    /// [`<figcaption>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/figcaption)
    /// element.
    figcaption {};

    /// Build a
    /// [`<figure>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/figure)
    /// element.
    figure {};

    /// Build a
    /// [`<hr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/hr)
    /// element.
    hr {};

    /// Build a
    /// [`<li>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/li)
    /// element.
    li {
        value: isize,
    };

    /// Build a
    /// [`<ol>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ol)
    /// element.
    ol {
        reversed: Bool,
        start: isize,
        r#type: OrderedListType,
    };

    /// Build a
    /// [`<p>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/p)
    /// element.
    p {};

    /// Build a
    /// [`<pre>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/pre)
    /// element.
    pre {};

    /// Build a
    /// [`<ul>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ul)
    /// element.
    ul {};


    // Inline text semantics

    /// Build a
    /// [`<a>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/a)
    /// element.
    a {
        download: String,
        href: Uri,
        hreflang: LanguageTag,
        target: Target,
        r#type: Mime,
        // ping: SpacedList<Uri>,
        // rel: SpacedList<LinkType>,
        ping: SpacedList,
        rel: SpacedList,
    };

    /// Build a
    /// [`<abbr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/abbr)
    /// element.
    abbr {};

    /// Build a
    /// [`<b>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/b)
    /// element.
    b {};

    /// Build a
    /// [`<bdi>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/bdi)
    /// element.
    bdi {};

    /// Build a
    /// [`<bdo>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/bdo)
    /// element.
    bdo {};

    /// Build a
    /// [`<br>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/br)
    /// element.
    br {};

    /// Build a
    /// [`<cite>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/cite)
    /// element.
    cite {};

    /// Build a
    /// [`<code>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/code)
    /// element.
    code {
        language: String,
    };

    /// Build a
    /// [`<data>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/data)
    /// element.
    data {
        value: String,
    };

    /// Build a
    /// [`<dfn>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dfn)
    /// element.
    dfn {};

    /// Build a
    /// [`<em>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/em)
    /// element.
    em {};

    /// Build a
    /// [`<i>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/i)
    /// element.
    i {};

    /// Build a
    /// [`<kbd>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/kbd)
    /// element.
    kbd {};

    /// Build a
    /// [`<mark>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/mark)
    /// element.
    mark {};

    /// Build a
    /// [`<menu>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/menu)
    /// element.
    menu {};

    /// Build a
    /// [`<q>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/q)
    /// element.
    q {
        cite: Uri,
    };


    /// Build a
    /// [`<rp>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/rp)
    /// element.
    rp {};


    /// Build a
    /// [`<rt>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/rt)
    /// element.
    rt {};


    /// Build a
    /// [`<ruby>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ruby)
    /// element.
    ruby {};

    /// Build a
    /// [`<s>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/s)
    /// element.
    s {};

    /// Build a
    /// [`<samp>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/samp)
    /// element.
    samp {};

    /// Build a
    /// [`<small>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/small)
    /// element.
    small {};

    /// Build a
    /// [`<span>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/span)
    /// element.
    span {};

    /// Build a
    /// [`<strong>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/strong)
    /// element.
    strong {};

    /// Build a
    /// [`<sub>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/sub)
    /// element.
    sub {};

    /// Build a
    /// [`<sup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/sup)
    /// element.
    sup {};

    /// Build a
    /// [`<time>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/time)
    /// element.
    time {};

    /// Build a
    /// [`<u>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/u)
    /// element.
    u {};

    /// Build a
    /// [`<var>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/var)
    /// element.
    var {};

    /// Build a
    /// [`<wbr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/wbr)
    /// element.
    wbr {};


    // Image and multimedia

    /// Build a
    /// [`<area>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/area)
    /// element.
    area {
        alt: String,
        coords: String, // TODO could perhaps be validated
        download: Bool,
        href: Uri,
        hreflang: LanguageTag,
        shape: AreaShape,
        target: Target,
        // ping: SpacedList<Uri>,
        // rel: SpacedSet<LinkType>,
    };

    /// Build a
    /// [`<audio>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/audio)
    /// element.
    audio {
        autoplay: Bool,
        controls: Bool,
        crossorigin: CrossOrigin,
        muted: Bool,
        preload: Preload,
        src: Uri,
        r#loop: Bool,
    };

    /// Build a
    /// [`<img>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/img)
    /// element.
    img {
        alt: String,
        crossorigin: CrossOrigin,
        decoding: ImageDecoding,
        height: usize,
        ismap: Bool,
        src: Uri,
        srcset: String, // FIXME this is much more complicated
        usemap: String, // FIXME should be a fragment starting with '#'
        width: usize,
        referrerpolicy: String,
        // sizes: SpacedList<String>, // FIXME it's not really just a string
    };

    /// Build a
    /// [`<map>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/map)
    /// element.
    map {
        name: Id,
    };

    /// Build a
    /// [`<track>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/track)
    /// element.
    track {
        default: Bool,
        kind: VideoKind,
        label: String,
        src: Uri,
        srclang: LanguageTag,
    };

    /// Build a
    /// [`<video>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/video)
    /// element.
    video {
        autoplay: Bool,
        controls: Bool,
        crossorigin: CrossOrigin,
        height: usize,
        r#loop: Bool,
        muted: Bool,
        preload: Preload,
        playsinline: Bool,
        poster: Uri,
        src: Uri,
        width: usize,
    };


    // Embedded content

    /// Build a
    /// [`<embed>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/embed)
    /// element.
    embed {
        height: usize,
        src: Uri,
        r#type: Mime,
        width: usize,
    };

    /// Build a
    /// [`<iframe>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/iframe)
    /// element.
    iframe {
        allow: FeaturePolicy,
        allowfullscreen: Bool,
        allowpaymentrequest: Bool,
        height: usize,
        name: Id,
        referrerpolicy: ReferrerPolicy,
        src: Uri,
        srcdoc: Uri,
        width: usize,

        marginWidth: String,
        align: String,
        longdesc: String,

        scrolling: String,
        marginHeight: String,
        frameBorder: String,
        // sandbox: SpacedSet<Sandbox>,
    };

    /// Build a
    /// [`<object>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/object)
    /// element.
    object {
        data: Uri,
        form: Id,
        height: usize,
        name: Id,
        r#type: Mime,
        typemustmatch: Bool,
        usemap: String, // TODO should be a fragment starting with '#'
        width: usize,
    };

    /// Build a
    /// [`<param>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/param)
    /// element.
    param {
        name: String,
        value: String,
    };

    /// Build a
    /// [`<picture>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/picture)
    /// element.
    picture {};

    /// Build a
    /// [`<source>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/source)
    /// element.
    source {
        src: Uri,
        r#type: Mime,
    };


    // Scripting

    /// Build a
    /// [`<canvas>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/canvas)
    /// element.
    canvas {
        height: usize,
        width: usize,
    };

    /// Build a
    /// [`<noscript>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/noscript)
    /// element.
    noscript {};

    /// Build a
    /// [`<script>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script)
    /// element.
    ///
    /// The [`script`] HTML element is used to embed executable code or data; this is typically used to embed or refer to
    /// JavaScript code. The [`script`] element can also be used with other languages, such as WebGL's GLSL shader
    /// programming language and JSON.
    script {
        /// Normal script elements pass minimal information to the window.onerror for scripts which do not pass the
        /// standard CORS checks. To allow error logging for sites which use a separate domain for static media, use
        /// this attribute. See CORS settings attributes for a more descriptive explanation of its valid arguments.
        crossorigin: CrossOrigin,

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
        defer: Bool,
        integrity: Integrity,
        nomodule: Bool,
        nonce: Nonce,
        src: Uri,
        text: String,

    };


    // Demarcating edits

    /// Build a
    /// [`<del>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/del)
    /// element.
    del {
        cite: Uri,
        datetime: Datetime,
    };

    /// Build a
    /// [`<ins>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ins)
    /// element.
    ins {
        cite: Uri,
        datetime: Datetime,
    };


    // Table content

    /// Build a
    /// [`<caption>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/caption)
    /// element.
    caption {};

    /// Build a
    /// [`<col>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/col)
    /// element.
    col {
        span: usize,
    };

    /// Build a
    /// [`<colgroup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/colgroup)
    /// element.
    colgroup {
        span: usize,
    };

    /// Build a
    /// [`<table>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/table)
    /// element.
    table {};

    /// Build a
    /// [`<tbody>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tbody)
    /// element.
    tbody {};

    /// Build a
    /// [`<td>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/td)
    /// element.
    td {
        colspan: usize,
        rowspan: usize,
        // headers: SpacedSet<Id>,
    };

    /// Build a
    /// [`<tfoot>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tfoot)
    /// element.
    tfoot {};

    /// Build a
    /// [`<th>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/th)
    /// element.
    th {
        abbr: String,
        colspan: usize,
        rowspan: usize,
        scope: TableHeaderScope,
        // headers: SpacedSet<Id>,
    };

    /// Build a
    /// [`<thead>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/thead)
    /// element.
    thead {};

    /// Build a
    /// [`<tr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tr)
    /// element.
    tr {};


    // Forms

    /// Build a
    /// [`<button>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/button)
    /// element.
    button {
        autofocus: Bool,
        disabled: Bool,
        form: Id,
        formaction: Uri,
        formenctype: FormEncodingType,
        formmethod: FormMethod,
        formnovalidate: Bool,
        formtarget: Target,
        name: Id,
        value: String,
    };

    /// Build a
    /// [`<datalist>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/datalist)
    /// element.
    datalist {};

    /// Build a
    /// [`<fieldset>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/fieldset)
    /// element.
    fieldset {};

    /// Build a
    /// [`<form>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
    /// element.
    form {
        // accept-charset: SpacedList<CharacterEncoding>,
        action: Uri,
        autocomplete: OnOff,
        enctype: FormEncodingType,
        method: FormMethod,
        name: Id,
        novalidate: Bool,
        target: Target,
    };

    /// Build a
    /// [`<input>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input)
    /// element.
    input {
        accept: String,
        alt: String,
        autocomplete: String,
        autofocus: Bool,
        capture: String,
        checked: Bool,
        disabled: Bool,
        form: Id,
        formaction: Uri,
        formenctype: FormEncodingType,
        formmethod: FormDialogMethod,
        formnovalidate: Bool,
        formtarget: Target,
        height: isize,
        list: Id,
        max: String,
        maxlength: usize,
        min: String,
        minlength: usize,
        multiple: Bool,
        name: Id,
        pattern: String,
        placeholder: String,
        readonly: Bool,
        required: Bool,
        size: usize,
        spellcheck: Bool,
        src: Uri,
        step: String,
        tabindex: usize,
        width: isize,

        // Manual implementations below...
        // r#type: InputType,
        // value: String,
    };

    /// Build a
    /// [`<label>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/label)
    /// element.
    label {
        form: Id,
        // r#for: Id,
    };

    /// Build a
    /// [`<legend>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/legend)
    /// element.
    legend {};

    /// Build a
    /// [`<meter>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meter)
    /// element.
    meter {
        value: isize,
        min: isize,
        max: isize,
        low: isize,
        high: isize,
        optimum: isize,
        form: Id,
    };

    /// Build a
    /// [`<optgroup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/optgroup)
    /// element.
    optgroup {
        disabled: Bool,
        label: String,
    };

    /// Build a
    /// [`<option>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/option)
    /// element.
    option {
        disabled: Bool,
        label: String,


        value: String,

        // defined below
        // selected: Bool,
    };

    /// Build a
    /// [`<output>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/output)
    /// element.
    output {
        form: Id,
        name: Id,
        // r#for: SpacedSet<Id>,
    };

    /// Build a
    /// [`<progress>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/progress)
    /// element.
    progress {
        max: f64,
        value: f64,
    };

    /// Build a
    /// [`<select>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/select)
    /// element.
    select {
        // defined below
        // value: String,
        autocomplete: String,
        autofocus: Bool,
        disabled: Bool,
        form: Id,
        multiple: Bool,
        name: Id,
        required: Bool,
        size: usize,
    };

    /// Build a
    /// [`<textarea>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/textarea)
    /// element.
    textarea {
        autocomplete: OnOff,
        autofocus: Bool,
        cols: usize,
        disabled: Bool,
        form: Id,
        maxlength: usize,
        minlength: usize,
        name: Id,
        placeholder: String,
        readonly: Bool,
        required: Bool,
        rows: usize,
        spellcheck: BoolOrDefault,
        wrap: Wrap,
    };


    // Interactive elements

    /// Build a
    /// [`<details>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/details)
    /// element.
    details {
        open: Bool,
    };



    /// Build a
    /// [`<summary>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/summary)
    /// element.
    summary {};

    // Web components

    /// Build a
    /// [`<slot>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/slot)
    /// element.
    slot {};

    /// Build a
    /// [`<template>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/template)
    /// element.
    template {};
}

builder_constructors! {
    // SVG components
    /// Build a
    /// [`<svg>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/svg)
    /// element.
    svg <> "http://www.w3.org/2000/svg" { };


    // /// Build a
    // /// [`<a>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/a)
    // /// element.
    // a <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<animate>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/animate)
    /// element.
    animate <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<animateMotion>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/animateMotion)
    /// element.
    animateMotion <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<animateTransform>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/animateTransform)
    /// element.
    animateTransform <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<circle>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/circle)
    /// element.
    circle <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<clipPath>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/clipPath)
    /// element.
    clipPath <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<defs>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/defs)
    /// element.
    defs <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<desc>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/desc)
    /// element.
    desc <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<discard>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/discard)
    /// element.
    discard <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<ellipse>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/ellipse)
    /// element.
    ellipse <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feBlend>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feBlend)
    /// element.
    feBlend <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feColorMatrix>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feColorMatrix)
    /// element.
    feColorMatrix <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feComponentTransfer>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feComponentTransfer)
    /// element.
    feComponentTransfer <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feComposite>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feComposite)
    /// element.
    feComposite <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feConvolveMatrix>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feConvolveMatrix)
    /// element.
    feConvolveMatrix <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feDiffuseLighting>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feDiffuseLighting)
    /// element.
    feDiffuseLighting <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feDisplacementMap>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feDisplacementMap)
    /// element.
    feDisplacementMap <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feDistantLight>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feDistantLight)
    /// element.
    feDistantLight <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feDropShadow>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feDropShadow)
    /// element.
    feDropShadow <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feFlood>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feFlood)
    /// element.
    feFlood <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feFuncA>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feFuncA)
    /// element.
    feFuncA <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feFuncB>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feFuncB)
    /// element.
    feFuncB <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feFuncG>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feFuncG)
    /// element.
    feFuncG <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feFuncR>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feFuncR)
    /// element.
    feFuncR <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feGaussianBlur>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feGaussianBlur)
    /// element.
    feGaussianBlur <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feImage>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feImage)
    /// element.
    feImage <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feMerge>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feMerge)
    /// element.
    feMerge <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feMergeNode>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feMergeNode)
    /// element.
    feMergeNode <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feMorphology>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feMorphology)
    /// element.
    feMorphology <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feOffset>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feOffset)
    /// element.
    feOffset <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<fePointLight>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/fePointLight)
    /// element.
    fePointLight <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feSpecularLighting>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feSpecularLighting)
    /// element.
    feSpecularLighting <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feSpotLight>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feSpotLight)
    /// element.
    feSpotLight <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feTile>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feTile)
    /// element.
    feTile <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<feTurbulence>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feTurbulence)
    /// element.
    feTurbulence <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<filter>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/filter)
    /// element.
    filter <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<foreignObject>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/foreignObject)
    /// element.
    foreignObject <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<g>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/g)
    /// element.
    g <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<hatch>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/hatch)
    /// element.
    hatch <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<hatchpath>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/hatchpath)
    /// element.
    hatchpath <> "http://www.w3.org/2000/svg" {};

    // /// Build a
    // /// [`<image>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/image)
    // /// element.
    // image <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<line>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/line)
    /// element.
    line <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<linearGradient>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/linearGradient)
    /// element.
    linearGradient <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<marker>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/marker)
    /// element.
    marker <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<mask>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/mask)
    /// element.
    mask <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<metadata>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/metadata)
    /// element.
    metadata <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<mpath>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/mpath)
    /// element.
    mpath <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<path>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/path)
    /// element.
    path <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<pattern>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/pattern)
    /// element.
    pattern <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<polygon>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/polygon)
    /// element.
    polygon <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<polyline>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/polyline)
    /// element.
    polyline <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<radialGradient>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/radialGradient)
    /// element.
    radialGradient <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<rect>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/rect)
    /// element.
    rect <> "http://www.w3.org/2000/svg" {};

    // /// Build a
    // /// [`<script>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/script)
    // /// element.
    // script <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<set>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/set)
    /// element.
    set <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<stop>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/stop)
    /// element.
    stop <> "http://www.w3.org/2000/svg" {};

    // /// Build a
    // /// [`<style>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/style)
    // /// element.
    // style <> "http://www.w3.org/2000/svg" {};

    // /// Build a
    // /// [`<svg>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/svg)
    // /// element.
    // svg <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<switch>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/switch)
    /// element.
    switch <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<symbol>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/symbol)
    /// element.
    symbol <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<text>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/text)
    /// element.
    text <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<textPath>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/textPath)
    /// element.
    textPath <> "http://www.w3.org/2000/svg" {};

    // /// Build a
    // /// [`<title>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/title)
    // /// element.
    // title <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<tspan>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/tspan)
    /// element.
    tspan <> "http://www.w3.org/2000/svg" {};

    /// Build a
    /// [`<view>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/view)
    /// element.
    view <> "http://www.w3.org/2000/svg" {};

    // /// Build a
    // /// [`<use>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/use)
    // /// element.
    // use <> "http://www.w3.org/2000/svg" {};


}
