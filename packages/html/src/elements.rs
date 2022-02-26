use crate::builder::ElementBuilder;
use dioxus_core::ScopeState;

macro_rules! builder_constructors {
    (
        $(
            $(#[$attr:meta])*
            $name:ident($concrete:ident) {
                $(
                    $(#[$attr_method:meta])*
                    $fil:ident: $vil:ident,
                )*
            };
         )*
    ) => {
        $(

            $(#[$attr])*
            pub fn $name(cx: &ScopeState) -> ElementBuilder {
                ElementBuilder::new(cx, stringify!($name))
            }

        )*
    };

    ( $(
        $(#[$attr:meta])*
        $name:ident($concrete:ident) <> $namespace:tt {
            $($fil:ident: $vil:ident,)*
        };
    )* ) => {
        $(
            $(#[$attr])*
            pub fn $name(cx: &ScopeState) -> ElementBuilder {
                ElementBuilder::new(cx, stringify!($name))
            }
        )*
    };
}

// Organized in the same order as
// https://developer.mozilla.org/en-US/docs/Web/HTML/Element
//
// Does not include obsolete elements.
//
// This namespace represents a collection of modern HTML-5 compatiable elements.
//
// This list does not include obsolete, deprecated, experimental, or poorly supported elements.
builder_constructors! {

    /// Build a
    /// [`<base>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/base)
    /// element
    base(Base) {
        href: Uri,
        target: Target,
    };

    /// Build a
    /// [`<head>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/head)
    /// element
    head(Head) {};

    /// Build a
    /// [`<link>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link)
    /// element
    link(Link) {
        // as: Mime,
        crossorigin: CrossOrigin,
        // href: Uri,
        hreflang: LanguageTag,
        media: String, // FIXME media query
        rel: LinkType,
        sizes: String, // FIXME
        // title: String, // FIXME
        r#type: Mime,
        integrity: String,
    };

    /// Build a
    /// [`<meta>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta)
    /// element
    meta(Meta) {
        charset: String, // FIXME IANA standard names
        // content: String,
        http_equiv: HTTPEquiv,
        name: Metadata,
    };

    /// Build a
    /// [`<style>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/style)
    /// element
    style(Style) {
        // r#type: Mime,
        // media: String, // FIXME media query
        nonce: Nonce,
        // title: String, // FIXME
    };

    /// Build a
    /// [`<title>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/title)
    /// element
    title(Title) { };

    /// Build a
    /// [`<body>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/body)
    /// element
    body(Body) {};

    /// Build a
    /// [`<address>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/address)
    /// element
    address(Address) {};

    /// Build a
    /// [`<article>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/article)
    /// element
    article(Article) {};

    /// Build a
    /// [`<aside>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/aside)
    /// element
    aside(Aside) {};

    /// Build a
    /// [`<footer>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/footer)
    /// element
    footer(Footer) {};

    /// Build a
    /// [`<header>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/header)
    /// element
    header(Header) {};

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

    /// Build a
    /// [`<h1>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h1)
    /// element
    h1(H1) {};

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

    /// Build a
    /// [`<h2>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h2)
    /// element
    h2(H2) {};

    ///
    /// # About
    /// - The HTML <h1> element is found within the <body> tag.
    /// - Headings can range from <h1> to <h6>.
    /// - The most important heading is <h1> and the least important heading is <h6>.
    /// - The <h1> heading is the first heading in the document.
    /// - The <h1> heading is usually a large bolded font.

    /// Build a
    /// [`<h3>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h3)
    /// element
    h3(H3) {};

    /// Build a
    /// [`<h4>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h4)
    /// element
    h4(H4) {};

    /// Build a
    /// [`<h5>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h5)
    /// element
    h5(H5) {};

    /// Build a
    /// [`<h6>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h6)
    /// element
    h6(H6) {};

    /// Build a
    /// [`<main>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/main)
    /// element
    main(Main) {};

    /// Build a
    /// [`<nav>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/nav)
    /// element
    nav(Nav) {};

    /// Build a
    /// [`<section>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/section)
    /// element
    section(Section) {};

    // Text content

    /// Build a
    /// [`<blockquote>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/blockquote)
    /// element
    blockquote(Blockquote) {
        cite: Uri,
    };

    /// Build a
    /// [`<dd>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dd)
    /// element
    dd(Dd) {};

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

    /// Build a
    /// [`<div>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/div)
    /// element
    div(Div) {};

    /// Build a
    /// [`<dl>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dl)
    /// element
    dl(Dl) {};

    /// Build a
    /// [`<dt>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dt)
    /// element
    dt(Dt) {};

    /// Build a
    /// [`<figcaption>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/figcaption)
    /// element
    figcaption(Figcaption) {};

    /// Build a
    /// [`<figure>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/figure)
    /// element
    figure(Figure) {};

    /// Build a
    /// [`<hr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/hr)
    /// element
    hr(Hr) {};

    /// Build a
    /// [`<li>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/li)
    /// element
    li(Li) {
        value: isize,
    };

    /// Build a
    /// [`<ol>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ol)
    /// element
    ol(Ol) {
        reversed: Bool,
        start: isize,
        r#type: OrderedListType,
    };

    /// Build a
    /// [`<p>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/p)
    /// element
    p(P) {};

    /// Build a
    /// [`<pre>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/pre)
    /// element
    pre(Pre) {};

    /// Build a
    /// [`<ul>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ul)
    /// element
    ul(Ul) {};

    // Inline text semantics

    /// Build a
    /// [`<a>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/a)
    /// element
    a(A) {
        download: String,
        // href: Uri,
        // hreflang: LanguageTag,
        // target: Target,
        // r#type: Mime,
        ping: SpacedList,
        rel: SpacedList,
        // ping: SpacedList<Uri>,
        // rel: SpacedList<LinkType>,
    };

    /// Build a
    /// [`<abbr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/abbr)
    /// element
    abbr(Abbr) {};

    /// Build a
    /// [`<b>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/b)
    /// element
    b(B) {};

    /// Build a
    /// [`<bdi>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/bdi)
    /// element
    bdi(Bdi) {};

    /// Build a
    /// [`<bdo>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/bdo)
    /// element
    bdo(Bdo) {};

    /// Build a
    /// [`<br>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/br)
    /// element
    br(Br) {};

    /// Build a
    /// [`<cite>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/cite)
    /// element
    cite(Cite) {};

    /// Build a
    /// [`<code>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/code)
    /// element
    code(Code) {
        language: String,
    };

    /// Build a
    /// [`<data>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/data)
    /// element
    data(Data) {
        value: String,
    };

    /// Build a
    /// [`<dfn>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dfn)
    /// element
    dfn(Dfn) {};

    /// Build a
    /// [`<em>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/em)
    /// element
    em(Em) {};

    /// Build a
    /// [`<i>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/i)
    /// element
    i(I) {};

    /// Build a
    /// [`<kbd>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/kbd)
    /// element
    kbd(Kbd) {};

    /// Build a
    /// [`<mark>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/mark)
    /// element
    mark(Mark) {};

    /// Build a
    /// [`<q>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/q)
    /// element
    q(Q) {
        cite: Uri,
    };

    /// Build a
    /// [`<rp>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/rp)
    /// element
    rp(Rp) {};

    /// Build a
    /// [`<rt>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/rt)
    /// element
    rt(Rt) {};

    /// Build a
    /// [`<ruby>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ruby)
    /// element
    ruby(Ruby) {};

    /// Build a
    /// [`<s>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/s)
    /// element
    s(S) {};

    /// Build a
    /// [`<samp>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/samp)
    /// element
    samp(Samp) {};

    /// Build a
    /// [`<small>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/small)
    /// element
    small(Small) {};

    /// Build a
    /// [`<span>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/span)
    /// element
    span(Span) {};

    /// Build a
    /// [`<strong>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/strong)
    /// element
    strong(Strong) {};

    /// Build a
    /// [`<sub>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/sub)
    /// element
    sub(Sub) {};

    /// Build a
    /// [`<sup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/sup)
    /// element
    sup(Sup) {};

    /// Build a
    /// [`<time>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/time)
    /// element
    time(Time) {};

    /// Build a
    /// [`<u>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/u)
    /// element
    u(U) {};

    /// Build a
    /// [`<var>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/var)
    /// element
    var(Var) {};

    /// Build a
    /// [`<wbr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/wbr)
    /// element
    wbr(Wbr) {};

    // Image and multimedia

    /// Build a
    /// [`<area>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/area)
    /// element
    area(Area) {
        alt: String,
        coords: String, // TODO could perhaps be validated
        download: Bool,
        // href: Uri,
        // hreflang: LanguageTag,
        shape: AreaShape,
        // target: Target,
        // ping: SpacedList<Uri>,
        // rel: SpacedSet<LinkType>,
    };

    /// Build a
    /// [`<audio>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/audio)
    /// element
    audio(Audio) {
        autoplay: Bool,
        controls: Bool,
        // crossorigin: CrossOrigin,
        muted: Bool,
        preload: Preload,
        src: Uri,
        r#loop: Bool,
    };

    /// Build a
    /// [`<img>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/img)
    /// element
    img(Img) {
        alt: String,
        // crossorigin: CrossOrigin,
        decoding: ImageDecoding,
        height_: usize,
        ismap: Bool,
        src: Uri,
        srcset: String, // FIXME this is much more complicated
        usemap: String, // FIXME should be a fragment starting with '#'
        width_: usize,
        referrerpolicy: String,
        // sizes: SpacedList<String>, // FIXME it's not really just a string
    };

    /// Build a
    /// [`<map>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/map)
    /// element
    map(Map) {
        name: Id,
    };

    /// Build a
    /// [`<track>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/track)
    /// element
    track(Track) {
        default: Bool,
        kind: VideoKind,
        label: String,
        src: Uri,
        srclang: LanguageTag,
    };

    /// Build a
    /// [`<video>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/video)
    /// element
    video(Video) {
        autoplay: Bool,
        controls: Bool,
        // crossorigin: CrossOrigin,
        height_: usize,
        r#loop: Bool,
        muted: Bool,
        preload: Preload,
        playsinline: Bool,
        poster: Uri,
        src: Uri,
        width_: usize,
    };

    // Embedded content

    /// Build a
    /// [`<embed>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/embed)
    /// element
    embed(Embed) {
        height_: usize,
        src: Uri,
        // r#type: Mime,
        width_: usize,
    };

    /// Build a
    /// [`<iframe>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/iframe)
    /// element
    iframe(Iframe) {
        allow: FeaturePolicy,
        allowfullscreen: Bool,
        allowpaymentrequest: Bool,
        height_: usize,
        name: Id,
        referrerpolicy: ReferrerPolicy,
        src: Uri,
        srcdoc: Uri,
        width_: usize,

        marginwidth_: String,
        align: String,
        longdesc: String,

        scrolling: String,
        marginheight_: String,
        frameBorder: String,
        // sandbox: SpacedSet<Sandbox>,
    };

    /// Build a
    /// [`<object>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/object)
    /// element
    object(Object) {
        // data: Uri,
        form: Id,
        height_: usize,
        name: Id,
        // r#type: Mime,
        typemustmatch: Bool,
        usemap: String, // TODO should be a fragment starting with '#'
        width_: usize,
    };

    /// Build a
    /// [`<param>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/param)
    /// element
    param(Param) {
        name: String,
        value: String,
    };

    /// Build a
    /// [`<picture>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/picture)
    /// element
    picture(Picture) {};

    /// Build a
    /// [`<source>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/source)
    /// element
    source(Source) {
        src: Uri,
        // r#type: Mime,
    };

    // Scripting

    /// Build a
    /// [`<canvas>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/canvas)
    /// element
    canvas(Canvas) {
        height_: usize,
        width_: usize,
    };

    /// Build a
    /// [`<noscript>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/noscript)
    /// element
    noscript(Noscript) {};
    ///
    /// The [`script`] HTML element is used to embed executable code or data; this is typically used to embed or refer to
    /// JavaScript code. The [`script`] element can also be used with other languages, such as WebGL's GLSL shader
    /// programming language and JSON.

    /// Build a
    /// [`<script>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script)
    /// element
    script(Script) {
        /// Normal script elements pass minimal information to the window.onerror for scripts which do not pass the
        /// standard CORS checks. To allow error logging for sites which use a separate domain for static media, use
        /// this attribute. See CORS settings attributes for a more descriptive explanation of its valid arguments.
        // crossorigin: CrossOrigin,

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
    /// element
    del(Del) {
        cite: Uri,
        datetime: Datetime,
    };

    /// Build a
    /// [`<ins>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ins)
    /// element
    ins(Ins) {
        cite: Uri,
        datetime: Datetime,
    };

    // Table content

    /// Build a
    /// [`<caption>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/caption)
    /// element
    caption(Caption) {};

    /// Build a
    /// [`<col>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/col)
    /// element
    col(Col) {
        span: usize,
    };

    /// Build a
    /// [`<colgroup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/colgroup)
    /// element
    colgroup(Colgroup) {
        span: usize,
    };

    /// Build a
    /// [`<table>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/table)
    /// element
    table(Table) {};

    /// Build a
    /// [`<tbody>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tbody)
    /// element
    tbody(Tbody) {};

    /// Build a
    /// [`<td>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/td)
    /// element
    td(Td) {
        colspan: usize,
        rowspan: usize,
        // headers: SpacedSet<Id>,
    };

    /// Build a
    /// [`<tfoot>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tfoot)
    /// element
    tfoot(Tfoot) {};

    /// Build a
    /// [`<th>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/th)
    /// element
    th(Th) {
        abbr: String,
        colspan: usize,
        rowspan: usize,
        scope: TableHeaderScope,
        // headers: SpacedSet<Id>,
    };

    /// Build a
    /// [`<thead>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/thead)
    /// element
    thead(Thead) {};

    /// Build a
    /// [`<tr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tr)
    /// element
    tr(Tr) {};

    // Forms

    /// Build a
    /// [`<button>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/button)
    /// element
    button(Button) {
        autofocus: Bool,
        disabled: Bool,
        form: Id,
        formaction: Uri,
        formenctype: FormEncodingType,
        formmethod: FormMethod,
        formnovalidate: Bool,
        // formtarget: Target,
        name: Id,
        value: String,
    };

    /// Build a
    /// [`<datalist>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/datalist)
    /// element
    datalist(Datalist) {};

    /// Build a
    /// [`<fieldset>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/fieldset)
    /// element
    fieldset(Fieldset) {};

    /// Build a
    /// [`<form>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
    /// element
    form(Form) {
        // accept-charset: SpacedList<CharacterEncoding>,
        action: Uri,
        autocomplete: OnOff,
        enctype: FormEncodingType,
        method: FormMethod,
        name: Id,
        novalidate: Bool,
        // target: Target,
    };

    /// Build a
    /// [`<input>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input)
    /// element
    input(Input) {
        accept: String,
        alt: String,
        autocomplete: String,
        autofocus: Bool,
        capture: String,
        checked: Bool,
        // disabled: Bool,
        form: Id,
        formaction: Uri,
        formenctype: FormEncodingType,
        formmethod: FormDialogMethod,
        formnovalidate: Bool,
        // formtarget: Target,
        height_: isize,
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
        // spellcheck: Bool,
        src: Uri,
        step: String,
        // tabindex: usize,
        width_: isize,

        // Manual implementations below...
        // r#type: InputType,
        // value: String,
    };

    /// Build a
    /// [`<label>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/label)
    /// element
    label(Label) {
        form: Id,
        // r#for: Id,
    };

    /// Build a
    /// [`<legend>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/legend)
    /// element
    legend(Legend) {};

    /// Build a
    /// [`<meter>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meter)
    /// element
    meter(Meter) {
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
    /// element
    optgroup(Optgroup) {
        // disabled: Bool,
        label: String,
    };

    /// Build a
    /// [`<option>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/option)
    /// element
    option(Option_) {
        // disabled: Bool,
        label: String,

        value: String,

        // defined below
        // selected: Bool,
    };

    /// Build a
    /// [`<output>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/output)
    /// element
    output(Output) {
        form: Id,
        name: Id,
        // r#for: SpacedSet<Id>,
    };

    /// Build a
    /// [`<progress>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/progress)
    /// element
    progress(Progress) {
        max: f64,
        value: f64,
    };

    /// Build a
    /// [`<select>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/select)
    /// element
    select(Select) {
        // defined below
        // value: String,
        autocomplete: String,
        autofocus: Bool,
        // disabled: Bool,
        form: Id,
        multiple: Bool,
        name: Id,
        required: Bool,
        size: usize,
    };

    /// Build a
    /// [`<textarea>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/textarea)
    /// element
    textarea(Textarea) {
        autocomplete: OnOff,
        autofocus: Bool,
        cols: usize,
        // disabled: Bool,
        form: Id,
        maxlength: usize,
        minlength: usize,
        name: Id,
        placeholder: String,
        readonly: Bool,
        required: Bool,
        rows: usize,
        // spellcheck: BoolOrDefault,
        wrap: Wrap,
    };

    // Interactive elements

    /// Build a
    /// [`<details>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/details)
    /// element
    details(Details) {
        open: Bool,
    };

    /// Build a
    /// [`<summary>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/summary)
    /// element
    summary(Summary) {};

    // Web components

    /// Build a
    /// [`<slot>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/slot)
    /// element
    slot(Slot) {};

    /// Build a
    /// [`<template>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/template)
    /// element
    template(Template) {};
}

pub mod svg_elements {
    use crate::builder::ElementBuilder;
    use dioxus_core::ScopeState;

    builder_constructors! {
        svg(Svg) <> "http://www.w3.org/2000/svg" { };
        animate(Animate) <> "http://www.w3.org/2000/svg" {};
        animateMotion(AnimateMotion) <> "http://www.w3.org/2000/svg" {};
        animateTransform(AnimateTransform) <> "http://www.w3.org/2000/svg" {};
        circle(Circle) <> "http://www.w3.org/2000/svg" {};
        clipPath(ClipPath) <> "http://www.w3.org/2000/svg" {};
        defs(Defs) <> "http://www.w3.org/2000/svg" {};
        desc(Desc) <> "http://www.w3.org/2000/svg" {};
        discard(Discard) <> "http://www.w3.org/2000/svg" {};
        ellipse(Ellipse) <> "http://www.w3.org/2000/svg" {};
        feBlend(FeBlend) <> "http://www.w3.org/2000/svg" {};
        feColorMatrix(FeColorMatrix) <> "http://www.w3.org/2000/svg" {};
        feComponentTransfer(FeComponentTransfer) <> "http://www.w3.org/2000/svg" {};
        feComposite(FeComposite) <> "http://www.w3.org/2000/svg" {};
        feConvolveMatrix(FeConvolveMatrix) <> "http://www.w3.org/2000/svg" {};
        feDiffuseLighting(FeDiffuseLighting) <> "http://www.w3.org/2000/svg" {};
        feDisplacementMap(FeDisplacementMap) <> "http://www.w3.org/2000/svg" {};
        feDistantLight(FeDistantLight) <> "http://www.w3.org/2000/svg" {};
        feDropShadow(FeDropShadow) <> "http://www.w3.org/2000/svg" {};
        feFlood(FeFlood) <> "http://www.w3.org/2000/svg" {};
        feFuncA(FeFuncA) <> "http://www.w3.org/2000/svg" {};
        feFuncB(FeFuncB) <> "http://www.w3.org/2000/svg" {};
        feFuncG(FeFuncG) <> "http://www.w3.org/2000/svg" {};
        feFuncR(FeFuncR) <> "http://www.w3.org/2000/svg" {};
        feGaussianBlur(FeGaussianBlur) <> "http://www.w3.org/2000/svg" {};
        feImage(FeImage) <> "http://www.w3.org/2000/svg" {};
        feMerge(FeMerge) <> "http://www.w3.org/2000/svg" {};
        feMergeNode(FeMergeNode) <> "http://www.w3.org/2000/svg" {};
        feMorphology(FeMorphology) <> "http://www.w3.org/2000/svg" {};
        feOffset(FeOffset) <> "http://www.w3.org/2000/svg" {};
        fePointLight(FePointLight) <> "http://www.w3.org/2000/svg" {};
        feSpecularLighting(FeSpecularLighting) <> "http://www.w3.org/2000/svg" {};
        feSpotLight(FeSpotLight) <> "http://www.w3.org/2000/svg" {};
        feTile(FeTile) <> "http://www.w3.org/2000/svg" {};
        feTurbulence(FeTurbulence) <> "http://www.w3.org/2000/svg" {};
        filter(Filter) <> "http://www.w3.org/2000/svg" {};
        foreignObject(ForeignObject) <> "http://www.w3.org/2000/svg" {};
        g(G) <> "http://www.w3.org/2000/svg" {};
        hatch(Hatch) <> "http://www.w3.org/2000/svg" {};
        hatchpath(Hatchpath) <> "http://www.w3.org/2000/svg" {};
        line(Line) <> "http://www.w3.org/2000/svg" {};
        linearGradient(LinearGradient) <> "http://www.w3.org/2000/svg" {};
        marker(Marker) <> "http://www.w3.org/2000/svg" {};
        mask(Mask) <> "http://www.w3.org/2000/svg" {};
        metadata(Metadata) <> "http://www.w3.org/2000/svg" {};
        mpath(Mpath) <> "http://www.w3.org/2000/svg" {};
        path(Path) <> "http://www.w3.org/2000/svg" {};
        pattern(Pattern) <> "http://www.w3.org/2000/svg" {};
        polygon(Polygon) <> "http://www.w3.org/2000/svg" {};
        polyline(Polyline) <> "http://www.w3.org/2000/svg" {};
        radialGradient(RadialGradient) <> "http://www.w3.org/2000/svg" {};
        rect(Rect) <> "http://www.w3.org/2000/svg" {};
        set(Set) <> "http://www.w3.org/2000/svg" {};
        stop(Stop) <> "http://www.w3.org/2000/svg" {};
        switch(Switch) <> "http://www.w3.org/2000/svg" {};
        symbol(Symbol) <> "http://www.w3.org/2000/svg" {};
        text(Text) <> "http://www.w3.org/2000/svg" {};
        textPath(TextPath) <> "http://www.w3.org/2000/svg" {};
        tspan(Tspan) <> "http://www.w3.org/2000/svg" {};
        view(View) <> "http://www.w3.org/2000/svg" {};

        // a <> "http://www.w3.org/2000/svg" {};
        // image <> "http://www.w3.org/2000/svg" {};
        // script <> "http://www.w3.org/2000/svg" {};
        // style <> "http://www.w3.org/2000/svg" {};
        // svg <> "http://www.w3.org/2000/svg" {};
        // title <> "http://www.w3.org/2000/svg" {};
        // use <> "http://www.w3.org/2000/svg" {};
    }
}
