#![allow(non_upper_case_globals, non_camel_case_types)]

use dioxus_core::prelude::dioxus_elements;
use dioxus_core::AttributeDescription;

dioxus_custom_element_macro::custom_elements! {
    HtmlElements;
    RawElement;

    /// Build a
    /// [`<base>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/base)
    /// element.
    base None {
        href: Uri DEFAULT,
        target: Target DEFAULT,
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
        crossorigin: CrossOrigin DEFAULT,
        href: Uri DEFAULT,
        hreflang: LanguageTag DEFAULT,
        media: String DEFAULT, // FIXME media query
        rel: LinkType DEFAULT,
        sizes: String DEFAULT, // FIXME
        // title: String DEFAULT, // FIXME
        r#type: Mime DEFAULT,
        integrity: String DEFAULT,
    };

    /// Build a
    /// [`<meta>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta)
    /// element.
    meta None {
        charset: String DEFAULT, // FIXME IANA standard names
        content: String DEFAULT,
        http_equiv: HTTPEquiv DEFAULT,
        name: Metadata DEFAULT,
    };

    /// Build a
    /// [`<style>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/style)
    /// element.
    style None {
        r#type: Mime DEFAULT,
        media: String DEFAULT, // FIXME media query
        nonce: Nonce DEFAULT,
        // title: String DEFAULT, // FIXME
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
    /// ```rust, ignore
    /// html!(<h1> A header element </h1>)
    /// rsx!(h1 { "A header element" })
    /// LazyNodes::new(|f| f.el(h1).children([f.text("A header element")]).finish())
    /// ```
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
    ///
    /// # Usage
    /// ```rust, ignore
    /// html!(<h2> A header element </h2>)
    /// rsx!(h2 { "A header element" })
    /// LazyNodes::new(|f| f.el(h2).children([f.text("A header element")]).finish())
    /// ```
    h2 None {};

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
        cite: Uri DEFAULT,
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
    /// - The <div> tag defines a division or a section in an HTML document.
    /// - The <div> tag is used as a container for HTML elements - which is then styled with CSS or manipulated with  JavaScript.
    /// - The <div> tag is easily styled by using the class or id attribute.
    /// - Any sort of content can be put inside the <div> tag!
    ///
    /// Note: By default, browsers always place a line break before and after the <div> element.
    ///
    /// ## Usage
    /// ```rust, ignore
    /// html!(<div> A header element </div>)
    /// rsx!(div { "A header element" })
    /// LazyNodes::new(|f| f.element(div, &[], &[], &[], None))
    /// ```
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
        value: isize DEFAULT,
    };

    /// Build a
    /// [`<ol>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ol)
    /// element.
    ol None {
        reversed: Bool DEFAULT,
        start: isize DEFAULT,
        r#type: OrderedListType DEFAULT,
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
        download: String DEFAULT,
        href: Uri DEFAULT,
        hreflang: LanguageTag DEFAULT,
        target: Target DEFAULT,
        r#type: Mime DEFAULT,
        // ping: SpacedList<Uri>,
        // rel: SpacedList<LinkType>,
        ping: SpacedList DEFAULT,
        rel: SpacedList DEFAULT,
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
        language: String DEFAULT,
    };

    /// Build a
    /// [`<data>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/data)
    /// element.
    data None {
        value: String DEFAULT,
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
        cite: Uri DEFAULT,
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
        datetime: Datetime DEFAULT,
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
        alt: String DEFAULT,
        coords: String DEFAULT, // TODO could perhaps be validated
        download: Bool DEFAULT,
        href: Uri DEFAULT,
        hreflang: LanguageTag DEFAULT,
        shape: AreaShape DEFAULT,
        target: Target DEFAULT,
        // ping: SpacedList<Uri>,
        // rel: SpacedSet<LinkType>,
    };

    /// Build a
    /// [`<audio>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/audio)
    /// element.
    audio None {
        autoplay: Bool DEFAULT,
        controls: Bool DEFAULT,
        crossorigin: CrossOrigin DEFAULT,
        muted: Bool DEFAULT,
        preload: Preload DEFAULT,
        src: Uri DEFAULT,
        r#loop: Bool DEFAULT,
    };

    /// Build a
    /// [`<img>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/img)
    /// element.
    img None {
        alt: String DEFAULT,
        crossorigin: CrossOrigin DEFAULT,
        decoding: ImageDecoding DEFAULT,
        height: usize DEFAULT,
        ismap: Bool DEFAULT,
        src: Uri DEFAULT,
        srcset: String DEFAULT, // FIXME this is much more complicated
        usemap: String DEFAULT, // FIXME should be a fragment starting with '#'
        width: usize DEFAULT,
        referrerpolicy: String DEFAULT,
        // sizes: SpacedList<String>, // FIXME it's not really just a string
    };

    /// Build a
    /// [`<map>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/map)
    /// element.
    map None {
        name: Id DEFAULT,
    };

    /// Build a
    /// [`<track>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/track)
    /// element.
    track None {
        default: Bool DEFAULT,
        kind: VideoKind DEFAULT,
        label: String DEFAULT,
        src: Uri DEFAULT,
        srclang: LanguageTag DEFAULT,
    };

    /// Build a
    /// [`<video>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/video)
    /// element.
    video None {
        autoplay: Bool DEFAULT,
        controls: Bool DEFAULT,
        crossorigin: CrossOrigin DEFAULT,
        height: usize DEFAULT,
        r#loop: Bool DEFAULT,
        muted: Bool DEFAULT,
        preload: Preload DEFAULT,
        playsinline: Bool DEFAULT,
        poster: Uri DEFAULT,
        src: Uri DEFAULT,
        width: usize DEFAULT,
    };


    // Embedded content

    /// Build a
    /// [`<embed>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/embed)
    /// element.
    embed None {
        height: usize DEFAULT,
        src: Uri DEFAULT,
        r#type: Mime DEFAULT,
        width: usize DEFAULT,
    };

    /// Build a
    /// [`<iframe>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/iframe)
    /// element.
    iframe None {
        allow: FeaturePolicy DEFAULT,
        allowfullscreen: Bool DEFAULT,
        allowpaymentrequest: Bool DEFAULT,
        height: usize DEFAULT,
        name: Id DEFAULT,
        referrerpolicy: ReferrerPolicy DEFAULT,
        src: Uri DEFAULT,
        srcdoc: Uri DEFAULT,
        width: usize DEFAULT,

        marginWidth: String DEFAULT,
        align: String DEFAULT,
        longdesc: String DEFAULT,

        scrolling: String DEFAULT,
        marginHeight: String DEFAULT,
        frameBorder: String DEFAULT,
        // sandbox: SpacedSet<Sandbox>,
    };

    /// Build a
    /// [`<object>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/object)
    /// element.
    object None {
        data: Uri DEFAULT,
        form: Id DEFAULT,
        height: usize DEFAULT,
        name: Id DEFAULT,
        r#type: Mime DEFAULT,
        typemustmatch: Bool DEFAULT,
        usemap: String DEFAULT, // TODO should be a fragment starting with '#'
        width: usize DEFAULT,
    };

    /// Build a
    /// [`<param>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/param)
    /// element.
    param None {
        name: String DEFAULT,
        value: String DEFAULT,
    };

    /// Build a
    /// [`<picture>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/picture)
    /// element.
    picture None {};

    /// Build a
    /// [`<source>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/source)
    /// element.
    source None {
        src: Uri DEFAULT,
        r#type: Mime DEFAULT,
    };


    // Scripting

    /// Build a
    /// [`<canvas>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/canvas)
    /// element.
    canvas None {
        height: usize DEFAULT,
        width: usize DEFAULT,
    };

    /// Build a
    /// [`<noscript>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/noscript)
    /// element.
    noscript None {};

    /// Build a
    /// [`<script>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script)
    /// element.
    ///
    /// The [`script`] HTML element is used to embed executable code or data; this is typically used to embed or refer to
    /// JavaScript code. The [`script`] element can also be used with other languages, such as WebGL's GLSL shader
    /// programming language and JSON.
    script None {
        /// Normal script elements pass minimal information to the window.onerror for scripts which do not pass the
        /// standard CORS checks. To allow error logging for sites which use a separate domain for static media, use
        /// this attribute. See CORS settings attributes for a more descriptive explanation of its valid arguments.
        crossorigin: CrossOrigin DEFAULT,

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
        defer: Bool DEFAULT,
        integrity: Integrity DEFAULT,
        nomodule: Bool DEFAULT,
        nonce: Nonce DEFAULT,
        src: Uri DEFAULT,
        text: String DEFAULT,

        // r#async: Bool,
        // r#type: String, // TODO could be an enum
        r#type: String "type",
        r#script: String "script",
    };


    // Demarcating edits

    /// Build a
    /// [`<del>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/del)
    /// element.
    del None {
        cite: Uri DEFAULT,
        datetime: Datetime DEFAULT,
    };

    /// Build a
    /// [`<ins>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ins)
    /// element.
    ins None {
        cite: Uri DEFAULT,
        datetime: Datetime DEFAULT,
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
        span: usize DEFAULT,
    };

    /// Build a
    /// [`<colgroup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/colgroup)
    /// element.
    colgroup None {
        span: usize DEFAULT,
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
        colspan: usize DEFAULT,
        rowspan: usize DEFAULT,
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
        abbr: String DEFAULT,
        colspan: usize DEFAULT,
        rowspan: usize DEFAULT,
        scope: TableHeaderScope DEFAULT,
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
        // autofocus: Bool DEFAULT,
        disabled: Bool DEFAULT,
        form: Id DEFAULT,
        formaction: Uri DEFAULT,
        formenctype: FormEncodingType DEFAULT,
        formmethod: FormMethod DEFAULT,
        formnovalidate: Bool DEFAULT,
        formtarget: Target DEFAULT,
        name: Id DEFAULT,
        value: String DEFAULT,
        r#type: String "type",
    };

    /// Build a
    /// [`<datalist>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/datalist)
    /// element.
    datalist None {};

    /// Build a
    /// [`<fieldset>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/fieldset)
    /// element.
    fieldset None {};

    /// Build a
    /// [`<form>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
    /// element.
    form None {
        // accept-charset: SpacedList<CharacterEncoding>,
        action: Uri DEFAULT,
        autocomplete: OnOff DEFAULT,
        enctype: FormEncodingType DEFAULT,
        method: FormMethod DEFAULT,
        name: Id DEFAULT,
        novalidate: Bool DEFAULT,
        target: Target DEFAULT,
    };

    /// Build a
    /// [`<input>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input)
    /// element.
    input None {
        accept: String DEFAULT,
        alt: String DEFAULT,
        autocomplete: String DEFAULT,
        // autofocus: Bool DEFAULT,
        capture: String DEFAULT,
        checked: Bool DEFAULT,
        disabled: Bool DEFAULT,
        form: Id DEFAULT,
        formaction: Uri DEFAULT,
        formenctype: FormEncodingType DEFAULT,
        formmethod: FormDialogMethod DEFAULT,
        formnovalidate: Bool DEFAULT,
        formtarget: Target DEFAULT,
        height: isize DEFAULT,
        list: Id DEFAULT,
        max: String DEFAULT,
        maxlength: usize DEFAULT,
        min: String DEFAULT,
        minlength: usize DEFAULT,
        multiple: Bool DEFAULT,
        name: Id DEFAULT,
        pattern: String DEFAULT,
        placeholder: String DEFAULT,
        readonly: Bool DEFAULT,
        required: Bool DEFAULT,
        // size: usize DEFAULT,
        // spellcheck: Bool DEFAULT,
        src: Uri DEFAULT,
        step: String DEFAULT,
        // tabindex: usize DEFAULT,
        width: isize DEFAULT,

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
        r#type: InputType "type",

        // value: String,
        value: String volatile,
    };

    /// Build a
    /// [`<label>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/label)
    /// element.
    label None {
        form: Id DEFAULT,
        r#for: Id "for",
    };

    /// Build a
    /// [`<legend>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/legend)
    /// element.
    legend None {};

    /// Build a
    /// [`<meter>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meter)
    /// element.
    meter None {
        value: isize DEFAULT,
        min: isize DEFAULT,
        max: isize DEFAULT,
        low: isize DEFAULT,
        high: isize DEFAULT,
        optimum: isize DEFAULT,
        form: Id DEFAULT,
    };

    /// Build a
    /// [`<optgroup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/optgroup)
    /// element.
    optgroup None {
        disabled: Bool DEFAULT,
        label: String DEFAULT,
    };

    /// Build a
    /// [`<option>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/option)
    /// element.
    option None {
        disabled: Bool DEFAULT,
        label: String DEFAULT,


        value: String DEFAULT,

        selected: Bool volatile,
    };

    /// Build a
    /// [`<output>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/output)
    /// element.
    output None {
        form: Id DEFAULT,
        name: Id DEFAULT,
        // r#for: SpacedSet<Id>,
    };

    /// Build a
    /// [`<progress>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/progress)
    /// element.
    progress None {
        max: f64 DEFAULT,
        value: f64 DEFAULT,
    };

    /// Build a
    /// [`<select>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/select)
    /// element.
    select None {
        // defined below
        // value: String,
        autocomplete: String DEFAULT,
        // autofocus: Bool DEFAULT,
        disabled: Bool DEFAULT,
        form: Id DEFAULT,
        multiple: Bool DEFAULT,
        name: Id DEFAULT,
        required: Bool DEFAULT,
        // size: usize DEFAULT,
        value: String volatile,
    };

    /// Build a
    /// [`<textarea>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/textarea)
    /// element.
    textarea None {
        autocomplete: OnOff DEFAULT,
        // autofocus: Bool DEFAULT,
        cols: usize DEFAULT,
        disabled: Bool DEFAULT,
        form: Id DEFAULT,
        maxlength: usize DEFAULT,
        minlength: usize DEFAULT,
        name: Id DEFAULT,
        placeholder: String DEFAULT,
        readonly: Bool DEFAULT,
        required: Bool DEFAULT,
        rows: usize DEFAULT,
        // spellcheck: BoolOrDefault DEFAULT,
        wrap: Wrap DEFAULT,
        value: Strign volatile,
    };


    // Interactive elements

    /// Build a
    /// [`<details>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/details)
    /// element.
    details None {
        open: Bool DEFAULT,
    };

    /// Build dialog
    /// [`<dialog>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dialog)
    /// element.
    dialog None {
        open: Bool DEFAULT,
    };

    /// Build a
    /// [`<summary>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/summary)
    /// element.
    summary None {};

    // Web components

    /// Build a
    /// [`<slot>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/slot)
    /// element.
    slot None {};

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

    // /// Build a
    // /// [`<image>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/image)
    // /// element.
    // image "http://www.w3.org/2000/svg" {};

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
    // use "http://www.w3.org/2000/svg" {};

    // Global attributes
    [
        /// Prevent the default action for this element.
        ///
        /// For more information, see the MDN docs:
        /// <https://developer.mozilla.org/en-US/docs/Web/API/Event/preventDefault>
        prevent_default: "dioxus-prevent-default";

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/accesskey>
        accesskey;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/autocapitalize>
        autocapitalize;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/autofocus>
        autofocus;

        /// The HTML class attribute is used to specify a class for an HTML element.
        ///
        /// ## Details
        /// Multiple HTML elements can share the same class.
        ///
        /// The class global attribute is a space-separated list of the case-sensitive classes of the element.
        /// Classes allow CSS and Javascript to select and access specific elements via the class selectors or
        /// functions like the DOM method document.getElementsByClassName.
        ///
        /// ## Example
        ///
        /// ### HTML:
        /// ```html
        /// <p class="note editorial">Above point sounds a bit obvious. Remove/rewrite?</p>
        /// ```
        ///
        /// ### CSS:
        /// ```css
        /// .note {
        ///     font-style: italic;
        ///     font-weight: bold;
        /// }
        ///
        /// .editorial {
        ///     background: rgb(255, 0, 0, .25);
        ///     padding: 10px;
        /// }
        /// ```
        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/class>
        class;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/contenteditable>
        contenteditable;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/data>
        data;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/dir>
        dir;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/draggable>
        draggable;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/enterkeyhint>
        enterkeyhint;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/exportparts>
        exportparts;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/hidden>
        hidden;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/id>
        id;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/inputmode>
        inputmode;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/is>
        is;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/itemid>
        itemid;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/itemprop>
        itemprop;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/itemref>
        itemref;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/itemscope>
        itemscope;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/itemtype>
        itemtype;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/lang>
        lang;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/nonce>
        nonce;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/part>
        part;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/role>
        role;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/slot>
        slot;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/spellcheck>
        spellcheck;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/style>
        style;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/tabindex>
        tabindex;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/title>
        title;

        /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/translate>
        translate;

        /// dangerous_inner_html is Dioxus's replacement for using innerHTML in the browser DOM. In general, setting
        /// HTML from code is risky because it’s easy to inadvertently expose your users to a cross-site scripting (XSS)
        /// attack. So, you can set HTML directly from Dioxus, but you have to type out dangerous_inner_html to remind
        /// yourself that it’s dangerous
        dangerous_inner_html;

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/align-content>
        align_content: "align-content", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/align-items>
        align_items: "align-items", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/align-self>
        align_self: "align-self", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/alignment-adjust>
        alignment_adjust: "alignment-adjust", "style";

        // /// <https://developer.mozilla.org/en-US/docs/Web/CSS/alignment-baseline>
        // alignment_baseline: "alignment-baseline", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/all>
        all: "all", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/alt>
        alt: "alt", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation>
        animation: "animation", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-delay>
        animation_delay: "animation-delay", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-direction>
        animation_direction: "animation-direction", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-duration>
        animation_duration: "animation-duration", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-fill-mode>
        animation_fill_mode: "animation-fill-mode", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-iteration-count>
        animation_iteration_count: "animation-iteration-count", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-name>
        animation_name: "animation-name", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-play-state>
        animation_play_state: "animation-play-state", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-timing-function>
        animation_timing_function: "animation-timing-function", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/azimuth>
        azimuth: "azimuth", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/backface-visibility>
        backface_visibility: "backface-visibility", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background>
        background: "background", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-attachment>
        background_attachment: "background-attachment", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-clip>
        background_clip: "background-clip", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-color>
        background_color: "background-color", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-image>
        background_image: "background-image", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-origin>
        background_origin: "background-origin", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-position>
        background_position: "background-position", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-repeat>
        background_repeat: "background-repeat", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-size>
        background_size: "background-size", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-blend-mode>
        background_blend_mode: "background-blend-mode", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/baseline-shift>
        baseline_shift: "baseline-shift", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/bleed>
        bleed: "bleed", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/bookmark-label>
        bookmark_label: "bookmark-label", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/bookmark-level>
        bookmark_level: "bookmark-level", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/bookmark-state>
        bookmark_state: "bookmark-state", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border>
        border: "border", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-color>
        border_color: "border-color", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-style>
        border_style: "border-style", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-width>
        border_width: "border-width", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom>
        border_bottom: "border-bottom", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom-color>
        border_bottom_color: "border-bottom-color", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom-style>
        border_bottom_style: "border-bottom-style", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom-width>
        border_bottom_width: "border-bottom-width", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-left>
        border_left: "border-left", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-left-color>
        border_left_color: "border-left-color", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-left-style>
        border_left_style: "border-left-style", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-left-width>
        border_left_width: "border-left-width", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-right>
        border_right: "border-right", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-right-color>
        border_right_color: "border-right-color", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-right-style>
        border_right_style: "border-right-style", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-right-width>
        border_right_width: "border-right-width", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top>
        border_top: "border-top", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top-color>
        border_top_color: "border-top-color", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top-style>
        border_top_style: "border-top-style", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top-width>
        border_top_width: "border-top-width", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-collapse>
        border_collapse: "border-collapse", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image>
        border_image: "border-image", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image-outset>
        border_image_outset: "border-image-outset", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image-repeat>
        border_image_repeat: "border-image-repeat", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image-slice>
        border_image_slice: "border-image-slice", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image-source>
        border_image_source: "border-image-source", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image-width>
        border_image_width: "border-image-width", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-radius>
        border_radius: "border-radius", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom-left-radius>
        border_bottom_left_radius: "border-bottom-left-radius", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom-right-radius>
        border_bottom_right_radius: "border-bottom-right-radius", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top-left-radius>
        border_top_left_radius: "border-top-left-radius", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top-right-radius>
        border_top_right_radius: "border-top-right-radius", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-spacing>
        border_spacing: "border-spacing", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/bottom>
        bottom: "bottom", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/box-decoration-break>
        box_decoration_break: "box-decoration-break", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/box-shadow>
        box_shadow: "box-shadow", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/box-sizing>
        box_sizing: "box-sizing", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/box-snap>
        box_snap: "box-snap", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/break-after>
        break_after: "break-after", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/break-before>
        break_before: "break-before", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/break-inside>
        break_inside: "break-inside", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/buffered-rendering>
        buffered_rendering: "buffered-rendering", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/caption-side>
        caption_side: "caption-side", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/clear>
        clear: "clear", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/clear-side>
        clear_side: "clear-side", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/clip>
        clip: "clip", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/clip-path>
        clip_path: "clip-path", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/clip-rule>
        clip_rule: "clip-rule", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color>
        color: "color", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-adjust>
        color_adjust: "color-adjust", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-correction>
        color_correction: "color-correction", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-interpolation>
        color_interpolation: "color-interpolation", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-interpolation-filters>
        color_interpolation_filters: "color-interpolation-filters", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-profile>
        color_profile: "color-profile", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-rendering>
        color_rendering: "color-rendering", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-fill>
        column_fill: "column-fill", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-gap>
        column_gap: "column-gap", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-rule>
        column_rule: "column-rule", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-rule-color>
        column_rule_color: "column-rule-color", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-rule-style>
        column_rule_style: "column-rule-style", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-rule-width>
        column_rule_width: "column-rule-width", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-span>
        column_span: "column-span", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/columns>
        columns: "columns", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-count>
        column_count: "column-count", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-width>
        column_width: "column-width", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/contain>
        contain: "contain", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/content>
        content: "content", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/counter-increment>
        counter_increment: "counter-increment", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/counter-reset>
        counter_reset: "counter-reset", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/counter-set>
        counter_set: "counter-set", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/cue>
        cue: "cue", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/cue-after>
        cue_after: "cue-after", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/cue-before>
        cue_before: "cue-before", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/cursor>
        cursor: "cursor", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/direction>
        direction: "direction", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/display>
        display: "display", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/display-inside>
        display_inside: "display-inside", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/display-outside>
        display_outside: "display-outside", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/display-extras>
        display_extras: "display-extras", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/display-box>
        display_box: "display-box", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/dominant-baseline>
        dominant_baseline: "dominant-baseline", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/elevation>
        elevation: "elevation", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/empty-cells>
        empty_cells: "empty-cells", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/enable-background>
        enable_background: "enable-background", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/fill>
        fill: "fill", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/fill-opacity>
        fill_opacity: "fill-opacity", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/fill-rule>
        fill_rule: "fill-rule", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/filter>
        filter: "filter", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/float>
        float: "float", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/float-defer-column>
        float_defer_column: "float-defer-column", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/float-defer-page>
        float_defer_page: "float-defer-page", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/float-offset>
        float_offset: "float-offset", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/float-wrap>
        float_wrap: "float-wrap", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flow-into>
        flow_into: "flow-into", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flow-from>
        flow_from: "flow-from", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex>
        flex: "flex", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-basis>
        flex_basis: "flex-basis", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-grow>
        flex_grow: "flex-grow", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-shrink>
        flex_shrink: "flex-shrink", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-flow>
        flex_flow: "flex-flow", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-direction>
        flex_direction: "flex-direction", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-wrap>
        flex_wrap: "flex-wrap", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flood-color>
        flood_color: "flood-color", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flood-opacity>
        flood_opacity: "flood-opacity", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font>
        font: "font", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-family>
        font_family: "font-family", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-size>
        font_size: "font-size", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-stretch>
        font_stretch: "font-stretch", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-style>
        font_style: "font-style", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-weight>
        font_weight: "font-weight", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-feature-settings>
        font_feature_settings: "font-feature-settings", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-kerning>
        font_kerning: "font-kerning", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-language-override>
        font_language_override: "font-language-override", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-size-adjust>
        font_size_adjust: "font-size-adjust", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-synthesis>
        font_synthesis: "font-synthesis", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant>
        font_variant: "font-variant", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-alternates>
        font_variant_alternates: "font-variant-alternates", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-caps>
        font_variant_caps: "font-variant-caps", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-east-asian>
        font_variant_east_asian: "font-variant-east-asian", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-ligatures>
        font_variant_ligatures: "font-variant-ligatures", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-numeric>
        font_variant_numeric: "font-variant-numeric", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-position>
        font_variant_position: "font-variant-position", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/footnote-policy>
        footnote_policy: "footnote-policy", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/glyph-orientation-horizontal>
        glyph_orientation_horizontal: "glyph-orientation-horizontal", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/glyph-orientation-vertical>
        glyph_orientation_vertical: "glyph-orientation-vertical", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid>
        grid: "grid", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-auto-flow>
        grid_auto_flow: "grid-auto-flow", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-auto-columns>
        grid_auto_columns: "grid-auto-columns", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-auto-rows>
        grid_auto_rows: "grid-auto-rows", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-template>
        grid_template: "grid-template", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-template-areas>
        grid_template_areas: "grid-template-areas", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-template-columns>
        grid_template_columns: "grid-template-columns", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-template-rows>
        grid_template_rows: "grid-template-rows", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-area>
        grid_area: "grid-area", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-column>
        grid_column: "grid-column", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-column-start>
        grid_column_start: "grid-column-start", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-column-end>
        grid_column_end: "grid-column-end", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-row>
        grid_row: "grid-row", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-row-start>
        grid_row_start: "grid-row-start", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-row-end>
        grid_row_end: "grid-row-end", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hanging-punctuation>
        hanging_punctuation: "hanging-punctuation", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/height>
        height: "height", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphenate-character>
        hyphenate_character: "hyphenate-character", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphenate-limit-chars>
        hyphenate_limit_chars: "hyphenate-limit-chars", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphenate-limit-last>
        hyphenate_limit_last: "hyphenate-limit-last", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphenate-limit-lines>
        hyphenate_limit_lines: "hyphenate-limit-lines", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphenate-limit-zone>
        hyphenate_limit_zone: "hyphenate-limit-zone", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphens>
        hyphens: "hyphens", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/icon>
        icon: "icon", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/image-orientation>
        image_orientation: "image-orientation", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/image-resolution>
        image_resolution: "image-resolution", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/image-rendering>
        image_rendering: "image-rendering", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ime>
        ime: "ime", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ime-align>
        ime_align: "ime-align", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ime-mode>
        ime_mode: "ime-mode", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ime-offset>
        ime_offset: "ime-offset", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ime-width>
        ime_width: "ime-width", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/initial-letters>
        initial_letters: "initial-letters", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/inline-box-align>
        inline_box_align: "inline-box-align", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/isolation>
        isolation: "isolation", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/justify-content>
        justify_content: "justify-content", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/justify-items>
        justify_items: "justify-items", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/justify-self>
        justify_self: "justify-self", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/kerning>
        kerning: "kerning", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/left>
        left: "left", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/letter-spacing>
        letter_spacing: "letter-spacing", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/lighting-color>
        lighting_color: "lighting-color", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-box-contain>
        line_box_contain: "line-box-contain", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-break>
        line_break: "line-break", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-grid>
        line_grid: "line-grid", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-height>
        line_height: "line-height", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-slack>
        line_slack: "line-slack", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-snap>
        line_snap: "line-snap", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/list-style>
        list_style: "list-style", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/list-style-image>
        list_style_image: "list-style-image", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/list-style-position>
        list_style_position: "list-style-position", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/list-style-type>
        list_style_type: "list-style-type", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/margin>
        margin: "margin", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/margin-bottom>
        margin_bottom: "margin-bottom", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/margin-left>
        margin_left: "margin-left", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/margin-right>
        margin_right: "margin-right", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/margin-top>
        margin_top: "margin-top", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker>
        marker: "marker", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-end>
        marker_end: "marker-end", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-mid>
        marker_mid: "marker-mid", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-pattern>
        marker_pattern: "marker-pattern", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-segment>
        marker_segment: "marker-segment", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-start>
        marker_start: "marker-start", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-knockout-left>
        marker_knockout_left: "marker-knockout-left", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-knockout-right>
        marker_knockout_right: "marker-knockout-right", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-side>
        marker_side: "marker-side", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marks>
        marks: "marks", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marquee-direction>
        marquee_direction: "marquee-direction", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marquee-play-count>
        marquee_play_count: "marquee-play-count", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marquee-speed>
        marquee_speed: "marquee-speed", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marquee-style>
        marquee_style: "marquee-style", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask>
        mask: "mask", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-image>
        mask_image: "mask-image", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-repeat>
        mask_repeat: "mask-repeat", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-position>
        mask_position: "mask-position", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-clip>
        mask_clip: "mask-clip", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-origin>
        mask_origin: "mask-origin", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-size>
        mask_size: "mask-size", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box>
        mask_box: "mask-box", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box-outset>
        mask_box_outset: "mask-box-outset", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box-repeat>
        mask_box_repeat: "mask-box-repeat", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box-slice>
        mask_box_slice: "mask-box-slice", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box-source>
        mask_box_source: "mask-box-source", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box-width>
        mask_box_width: "mask-box-width", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-type>
        mask_type: "mask-type", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/max-height>
        max_height: "max-height", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/max-lines>
        max_lines: "max-lines", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/max-width>
        max_width: "max-width", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/min-height>
        min_height: "min-height", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/min-width>
        min_width: "min-width", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mix-blend-mode>
        mix_blend_mode: "mix-blend-mode", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/nav-down>
        nav_down: "nav-down", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/nav-index>
        nav_index: "nav-index", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/nav-left>
        nav_left: "nav-left", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/nav-right>
        nav_right: "nav-right", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/nav-up>
        nav_up: "nav-up", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/object-fit>
        object_fit: "object-fit", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/object-position>
        object_position: "object-position", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/offset-after>
        offset_after: "offset-after", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/offset-before>
        offset_before: "offset-before", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/offset-end>
        offset_end: "offset-end", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/offset-start>
        offset_start: "offset-start", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/opacity>
        opacity: "opacity", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/order>
        order: "order", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/orphans>
        orphans: "orphans", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/outline>
        outline: "outline", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/outline-color>
        outline_color: "outline-color", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/outline-style>
        outline_style: "outline-style", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/outline-width>
        outline_width: "outline-width", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/outline-offset>
        outline_offset: "outline-offset", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/overflow>
        overflow: "overflow", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/overflow-x>
        overflow_x: "overflow-x", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/overflow-y>
        overflow_y: "overflow-y", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/overflow-style>
        overflow_style: "overflow-style", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/overflow-wrap>
        overflow_wrap: "overflow-wrap", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/padding>
        padding: "padding", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/padding-bottom>
        padding_bottom: "padding-bottom", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/padding-left>
        padding_left: "padding-left", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/padding-right>
        padding_right: "padding-right", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/padding-top>
        padding_top: "padding-top", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/page>
        page: "page", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/page-break-after>
        page_break_after: "page-break-after", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/page-break-before>
        page_break_before: "page-break-before", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/page-break-inside>
        page_break_inside: "page-break-inside", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/paint-order>
        paint_order: "paint-order", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pause>
        pause: "pause", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pause-after>
        pause_after: "pause-after", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pause-before>
        pause_before: "pause-before", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/perspective>
        perspective: "perspective", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/perspective-origin>
        perspective_origin: "perspective-origin", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pitch>
        pitch: "pitch", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pitch-range>
        pitch_range: "pitch-range", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/play-during>
        play_during: "play-during", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pointer-events>
        pointer_events: "pointer-events", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/position>
        position: "position", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/quotes>
        quotes: "quotes", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/region-fragment>
        region_fragment: "region-fragment", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/resize>
        resize: "resize", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/rest>
        rest: "rest", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/rest-after>
        rest_after: "rest-after", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/rest-before>
        rest_before: "rest-before", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/richness>
        richness: "richness", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/right>
        right: "right", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ruby-align>
        ruby_align: "ruby-align", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ruby-merge>
        ruby_merge: "ruby-merge", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ruby-position>
        ruby_position: "ruby-position", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-behavior>
        scroll_behavior: "scroll-behavior", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-snap-coordinate>
        scroll_snap_coordinate: "scroll-snap-coordinate", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-snap-destination>
        scroll_snap_destination: "scroll-snap-destination", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-snap-points-x>
        scroll_snap_points_x: "scroll-snap-points-x", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-snap-points-y>
        scroll_snap_points_y: "scroll-snap-points-y", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-snap-type>
        scroll_snap_type: "scroll-snap-type", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-image-threshold>
        shape_image_threshold: "shape-image-threshold", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-inside>
        shape_inside: "shape-inside", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-margin>
        shape_margin: "shape-margin", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-outside>
        shape_outside: "shape-outside", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-padding>
        shape_padding: "shape-padding", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-rendering>
        shape_rendering: "shape-rendering", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/size>
        size: "size", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speak>
        speak: "speak", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speak-as>
        speak_as: "speak-as", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speak-header>
        speak_header: "speak-header", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speak-numeral>
        speak_numeral: "speak-numeral", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speak-punctuation>
        speak_punctuation: "speak-punctuation", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speech-rate>
        speech_rate: "speech-rate", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stop-color>
        stop_color: "stop-color", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stop-opacity>
        stop_opacity: "stop-opacity", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stress>
        stress: "stress", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/string-set>
        string_set: "string-set", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke>
        stroke: "stroke", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-dasharray>
        stroke_dasharray: "stroke-dasharray", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-dashoffset>
        stroke_dashoffset: "stroke-dashoffset", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-linecap>
        stroke_linecap: "stroke-linecap", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-linejoin>
        stroke_linejoin: "stroke-linejoin", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-miterlimit>
        stroke_miterlimit: "stroke-miterlimit", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-opacity>
        stroke_opacity: "stroke-opacity", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-width>
        stroke_width: "stroke-width", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/tab-size>
        tab_size: "tab-size", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/table-layout>
        table_layout: "table-layout", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-align>
        text_align: "text-align", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-align-all>
        text_align_all: "text-align-all", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-align-last>
        text_align_last: "text-align-last", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-anchor>
        text_anchor: "text-anchor", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-combine-upright>
        text_combine_upright: "text-combine-upright", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration>
        text_decoration: "text-decoration", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration-color>
        text_decoration_color: "text-decoration-color", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration-line>
        text_decoration_line: "text-decoration-line", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration-style>
        text_decoration_style: "text-decoration-style", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration-skip>
        text_decoration_skip: "text-decoration-skip", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-emphasis>
        text_emphasis: "text-emphasis", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-emphasis-color>
        text_emphasis_color: "text-emphasis-color", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-emphasis-style>
        text_emphasis_style: "text-emphasis-style", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-emphasis-position>
        text_emphasis_position: "text-emphasis-position", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-emphasis-skip>
        text_emphasis_skip: "text-emphasis-skip", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-height>
        text_height: "text-height", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-indent>
        text_indent: "text-indent", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-justify>
        text_justify: "text-justify", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-orientation>
        text_orientation: "text-orientation", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-overflow>
        text_overflow: "text-overflow", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-rendering>
        text_rendering: "text-rendering", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-shadow>
        text_shadow: "text-shadow", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-size-adjust>
        text_size_adjust: "text-size-adjust", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-space-collapse>
        text_space_collapse: "text-space-collapse", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-spacing>
        text_spacing: "text-spacing", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-transform>
        text_transform: "text-transform", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-underline-position>
        text_underline_position: "text-underline-position", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-wrap>
        text_wrap: "text-wrap", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/top>
        top: "top", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/touch-action>
        touch_action: "touch-action", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transform>
        transform: "transform", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transform-box>
        transform_box: "transform-box", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transform-origin>
        transform_origin: "transform-origin", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transform-style>
        transform_style: "transform-style", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transition>
        transition: "transition", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transition-delay>
        transition_delay: "transition-delay", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transition-duration>
        transition_duration: "transition-duration", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transition-property>
        transition_property: "transition-property", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/unicode-bidi>
        unicode_bidi: "unicode-bidi", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/vector-effect>
        vector_effect: "vector-effect", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/vertical-align>
        vertical_align: "vertical-align", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/visibility>
        visibility: "visibility", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-balance>
        voice_balance: "voice-balance", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-duration>
        voice_duration: "voice-duration", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-family>
        voice_family: "voice-family", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-pitch>
        voice_pitch: "voice-pitch", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-range>
        voice_range: "voice-range", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-rate>
        voice_rate: "voice-rate", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-stress>
        voice_stress: "voice-stress", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-volumn>
        voice_volumn: "voice-volumn", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/volume>
        volume: "volume", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/white-space>
        white_space: "white-space", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/widows>
        widows: "widows", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/width>
        width: "width", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/will-change>
        will_change: "will-change", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/word-break>
        word_break: "word-break", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/word-spacing>
        word_spacing: "word-spacing", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/word-wrap>
        word_wrap: "word-wrap", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/wrap-flow>
        wrap_flow: "wrap-flow", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/wrap-through>
        wrap_through: "wrap-through", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/writing-mode>
        writing_mode: "writing-mode", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/gap>
        gap: "gap", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/list-style-type>
        list_styler_type: "list-style-type", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/row-gap>
        row_gap: "row-gap", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transition-timing-function>
        transition_timing_function: "transition-timing-function", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/user-select>
        user_select: "user-select", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/-webkit-user-select>
        webkit_user_select: "-webkit-user-select", "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/CSS/z-index>
        z_index : "z-index", "style";


    // Area attributes

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-current>
        aria_current: "aria-current";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-details>
        aria_details: "aria-details";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-disabled>
        aria_disabled: "aria-disabled";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-hidden>
        aria_hidden: "aria-hidden";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-invalid>
        aria_invalid: "aria-invalid";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-keyshortcuts>
        aria_keyshortcuts: "aria-keyshortcuts";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-label>
        aria_label: "aria-label";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-roledescription>
        aria_roledescription: "aria-roledescription";

    // Widget Attributes

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-autocomplete>
        aria_autocomplete: "aria-autocomplete";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-checked>
        aria_checked: "aria-checked";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-expanded>
        aria_expanded: "aria-expanded";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-haspopup>
        aria_haspopup: "aria-haspopup";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-level>
        aria_level: "aria-level";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-modal>
        aria_modal: "aria-modal";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-multiline>
        aria_multiline: "aria-multiline";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-multiselectable>
        aria_multiselectable: "aria-multiselectable";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-orientation>
        aria_orientation: "aria-orientation";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-placeholder>
        aria_placeholder: "aria-placeholder";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-pressed>
        aria_pressed: "aria-pressed";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-readonly>
        aria_readonly: "aria-readonly";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-required>
        aria_required: "aria-required";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-selected>
        aria_selected: "aria-selected";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-sort>
        aria_sort: "aria-sort";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-valuemax>
        aria_valuemax: "aria-valuemax";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-valuemin>
        aria_valuemin: "aria-valuemin";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-valuenow>
        aria_valuenow: "aria-valuenow";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-valuetext>
        aria_valuetext: "aria-valuetext";

    // Live Region Attributes

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-atomic>
        aria_atomic: "aria-atomic";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-busy>
        aria_busy: "aria-busy";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-live>
        aria_live: "aria-live";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-relevant>
        aria_relevant: "aria-relevant";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-dropeffect>
        aria_dropeffect: "aria-dropeffect";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-grabbed>
        aria_grabbed: "aria-grabbed";

    // Relationship Attributes

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-activedescendant>
        aria_activedescendant: "aria-activedescendant";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-colcount>
        aria_colcount: "aria-colcount";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-colindex>
        aria_colindex: "aria-colindex";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-colspan>
        aria_colspan: "aria-colspan";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-controls>
        aria_controls: "aria-controls";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-describedby>
        aria_describedby: "aria-describedby";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-errormessage>
        aria_errormessage: "aria-errormessage";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-flowto>
        aria_flowto: "aria-flowto";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-labelledby>
        aria_labelledby: "aria-labelledby";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-owns>
        aria_owns: "aria-owns";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-posinset>
        aria_posinset: "aria-posinset";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-rowcount>
        aria_rowcount: "aria-rowcount";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-rowindex>
        aria_rowindex: "aria-rowindex";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-rowspan>
        aria_rowspan: "aria-rowspan";

        /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-setsize>
        aria_setsize: "aria-setsize";


        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/accent-height>
        accent_height: "accent-height";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/accumulate>
        accumulate: "accumulate";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/additive>
        additive: "additive";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/alignment-baseline>
        alignment_baseline: "alignment-baseline";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/alphabetic>
        alphabetic: "alphabetic";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/amplitude>
        amplitude: "amplitude";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/arabic-form>
        arabic_form: "arabic-form";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/ascent>
        ascent: "ascent";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/attributeName>
        attributeName: "attributeName";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/attributeType>
        attributeType: "attributeType";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/azimuth>
        // azimuth: "azimuth";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/baseFrequency>
        baseFrequency: "baseFrequency";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/baseline-shift>
        // baseline_shift: "baseline-shift";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/baseProfile>
        baseProfile: "baseProfile";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/bbox>
        bbox: "bbox";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/begin>
        begin: "begin";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/bias>
        bias: "bias";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/by>
        by: "by";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/calcMode>
        calcMode: "calcMode";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/cap-height>
        cap_height: "cap-height";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/class>
        // class: "class";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/clip>
        // clip: "clip";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/clipPathUnits>
        clipPathUnits: "clipPathUnits";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/clip-path>
        // clip_path: "clip-path";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/clip-rule>
        // clip_rule: "clip-rule";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/color>
        // color: "color";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/color-interpolation>
        // color_interpolation: "color-interpolation";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/color-interpolation-filters>
        // color_interpolation_filters: "color-interpolation-filters";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/color-profile>
        // color_profile: "color-profile";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/color-rendering>
        // color_rendering: "color-rendering";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/contentScriptType>
        contentScriptType: "contentScriptType";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/contentStyleType>
        contentStyleType: "contentStyleType";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/crossorigin>
        // crossorigin: "crossorigin";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/cursor>
        // cursor: "cursor";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/cx>
        cx: "cx";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/cy>
        cy: "cy";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/d>
        d: "d";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/decelerate>
        decelerate: "decelerate";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/descent>
        descent: "descent";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/diffuseConstant>
        diffuseConstant: "diffuseConstant";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/direction>
        // direction: "direction";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/display>
        // display: "display";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/divisor>
        divisor: "divisor";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/dominant-baseline>
        // dominant_baseline: "dominant-baseline";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/dur>
        dur: "dur";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/dx>
        dx: "dx";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/dy>
        dy: "dy";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/edgeMode>
        edgeMode: "edgeMode";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/elevation>
        // elevation: "elevation";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/enable-background>
        // enable_background: "enable-background";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/end>
        end: "end";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/exponent>
        exponent: "exponent";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fill>
        // fill: "fill";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fill-opacity>
        // fill_opacity: "fill-opacity";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fill-rule>
        // fill_rule: "fill-rule";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/filter>
        // filter: "filter";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/filterRes>
        filterRes: "filterRes";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/filterUnits>
        filterUnits: "filterUnits";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/flood-color>
        // flood_color: "flood-color";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/flood-opacity>
        // flood_opacity: "flood-opacity";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/font-family>
        // font_family: "font-family";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/font-size>
        // font_size: "font-size";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/font-size-adjust>
        // font_size_adjust: "font-size-adjust";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/font-stretch>
        // font_stretch: "font-stretch";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/font-style>
        // font_style: "font-style";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/font-variant>
        // font_variant: "font-variant";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/font-weight>
        // font_weight: "font-weight";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/format>
        format: "format";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/from>
        from: "from";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fr>
        fr: "fr";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fx>
        fx: "fx";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fy>
        fy: "fy";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/g1>
        g1: "g1";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/g2>
        g2: "g2";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/glyph-name>
        glyph_name: "glyph-name";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/glyph-orientation-horizontal>
        // glyph_orientation_horizontal: "glyph-orientation-horizontal";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/glyph-orientation-vertical>
        // glyph_orientation_vertical: "glyph-orientation-vertical";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/glyphRef>
        glyphRef: "glyphRef";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/gradientTransform>
        gradientTransform: "gradientTransform";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/gradientUnits>
        gradientUnits: "gradientUnits";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/hanging>
        hanging: "hanging";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/height>
        height: "height";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/href>
        // href: "href";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/hreflang>
        // hreflang: "hreflang";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/horiz-adv-x>
        horiz_adv_x: "horiz-adv-x";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/horiz-origin-x>
        horiz_origin_x: "horiz-origin-x";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/id>
        // id: "id";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/ideographic>
        ideographic: "ideographic";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/image-rendering>
        // image_rendering: "image-rendering";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/_in>
        _in: "_in";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/in2>
        in2: "in2";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/intercept>
        intercept: "intercept";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/k>
        k: "k";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/k1>
        k1: "k1";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/k2>
        k2: "k2";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/k3>
        k3: "k3";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/k4>
        k4: "k4";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/kernelMatrix>
        kernelMatrix: "kernelMatrix";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/kernelUnitLength>
        kernelUnitLength: "kernelUnitLength";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/kerning>
        // kerning: "kerning";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/keyPoints>
        keyPoints: "keyPoints";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/keySplines>
        keySplines: "keySplines";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/keyTimes>
        keyTimes: "keyTimes";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/lang>
        // lang: "lang";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/lengthAdjust>
        lengthAdjust: "lengthAdjust";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/letter-spacing>
        // letter_spacing: "letter-spacing";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/lighting-color>
        // lighting_color: "lighting-color";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/limitingConeAngle>
        limitingConeAngle: "limitingConeAngle";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/local>
        local: "local";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/marker-end>
        // marker_end: "marker-end";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/marker-mid>
        // marker_mid: "marker-mid";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/marker_start>
        // marker_start: "marker_start";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/markerHeight>
        markerHeight: "markerHeight";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/markerUnits>
        markerUnits: "markerUnits";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/markerWidth>
        markerWidth: "markerWidth";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/mask>
        // mask: "mask";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/maskContentUnits>
        maskContentUnits: "maskContentUnits";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/maskUnits>
        maskUnits: "maskUnits";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/mathematical>
        mathematical: "mathematical";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/max>
        // max: "max";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/media>
        // media: "media";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/method>
        // method: "method";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/min>
        // min: "min";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/mode>
        mode: "mode";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/name>
        name: "name";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/numOctaves>
        numOctaves: "numOctaves";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/offset>
        offset: "offset";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/opacity>
        // opacity: "opacity";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/operator>
        operator: "operator";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/order>
        // order: "order";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/orient>
        orient: "orient";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/orientation>
        orientation: "orientation";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/origin>
        origin: "origin";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/overflow>
        // overflow: "overflow";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/overline-position>
        overline_position: "overline-position";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/overline-thickness>
        overline_thickness: "overline-thickness";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/panose-1>
        panose_1: "panose-1";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/paint-order>
        // paint_order: "paint-order";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/path>
        path: "path";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/pathLength>
        pathLength: "pathLength";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/patternContentUnits>
        patternContentUnits: "patternContentUnits";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/patternTransform>
        patternTransform: "patternTransform";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/patternUnits>
        patternUnits: "patternUnits";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/ping>
        ping: "ping";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/pointer-events>
        // pointer_events: "pointer-events";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/points>
        points: "points";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/pointsAtX>
        pointsAtX: "pointsAtX";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/pointsAtY>
        pointsAtY: "pointsAtY";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/pointsAtZ>
        pointsAtZ: "pointsAtZ";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/preserveAlpha>
        preserveAlpha: "preserveAlpha";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/preserveAspectRatio>
        preserveAspectRatio: "preserveAspectRatio";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/primitiveUnits>
        primitiveUnits: "primitiveUnits";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/r>
        r: "r";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/radius>
        radius: "radius";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/referrerPolicy>
        referrerPolicy: "referrerPolicy";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/refX>
        refX: "refX";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/refY>
        refY: "refY";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/rel>
        // rel: "rel";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/rendering-intent>
        rendering_intent: "rendering-intent";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/repeatCount>
        repeatCount: "repeatCount";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/repeatDur>
        repeatDur: "repeatDur";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/requiredExtensions>
        requiredExtensions: "requiredExtensions";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/requiredFeatures>
        requiredFeatures: "requiredFeatures";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/restart>
        restart: "restart";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/result>
        result: "result";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/role>
        // role: "role";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/rotate>
        rotate: "rotate";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/rx>
        rx: "rx";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/ry>
        ry: "ry";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/scale>
        scale: "scale";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/seed>
        seed: "seed";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/shape-rendering>
        // shape_rendering: "shape-rendering";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/slope>
        slope: "slope";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/spacing>
        spacing: "spacing";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/specularConstant>
        specularConstant: "specularConstant";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/specularExponent>
        specularExponent: "specularExponent";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/speed>
        speed: "speed";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/spreadMethod>
        spreadMethod: "spreadMethod";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/startOffset>
        startOffset: "startOffset";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stdDeviation>
        stdDeviation: "stdDeviation";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stemh>
        stemh: "stemh";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stemv>
        stemv: "stemv";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stitchTiles>
        stitchTiles: "stitchTiles";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stop_color>
        // stop_color: "stop_color";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stop_opacity>
        // stop_opacity: "stop_opacity";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/strikethrough-position>
        strikethrough_position: "strikethrough-position";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/strikethrough-thickness>
        strikethrough_thickness: "strikethrough-thickness";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/string>
        string: "string";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke>
        // stroke: "stroke";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-dasharray>
        // stroke_dasharray: "stroke-dasharray";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-dashoffset>
        // stroke_dashoffset: "stroke-dashoffset";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-linecap>
        // stroke_linecap: "stroke-linecap";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-linejoin>
        // stroke_linejoin: "stroke-linejoin";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-miterlimit>
        // stroke_miterlimit: "stroke-miterlimit";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-opacity>
        // stroke_opacity: "stroke-opacity";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-width>
        // stroke_width: "stroke-width";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/style>
        // style: "style";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/surfaceScale>
        surfaceScale: "surfaceScale";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/systemLanguage>
        systemLanguage: "systemLanguage";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/tabindex>
        // tabindex: "tabindex";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/tableValues>
        tableValues: "tableValues";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/target>
        // target: "target";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/targetX>
        targetX: "targetX";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/targetY>
        targetY: "targetY";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/text-anchor>
        // text_anchor: "text-anchor";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/text-decoration>
        // text_decoration: "text-decoration";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/text-rendering>
        // text_rendering: "text-rendering";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/textLength>
        textLength: "textLength";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/to>
        to: "to";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/transform>
        // transform: "transform";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/transform-origin>
        // transform_origin: "transform-origin";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/_type>
        // r#type: "_type";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/u1>
        u1: "u1";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/u2>
        u2: "u2";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/underline-position>
        underline_position: "underline-position";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/underline-thickness>
        underline_thickness: "underline-thickness";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/unicode>
        unicode: "unicode";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/unicode-bidi>
        // ade_bidi: "unicode-bidi";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/unicode-range>
        unicode_range: "unicode-range";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/units-per-em>
        units_per_em: "units-per-em";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/v-alphabetic>
        v_alphabetic: "v-alphabetic";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/v-hanging>
        v_hanging: "v-hanging";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/v-ideographic>
        v_ideographic: "v-ideographic";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/v-mathematical>
        v_mathematical: "v-mathematical";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/values>
        values: "values";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/vector-effect>
        // vector_effect: "vector-effect";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/version>
        version: "version";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/vert-adv-y>
        vert_adv_y: "vert-adv-y";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/vert-origin-x>
        vert_origin_x: "vert-origin-x";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/vert-origin-y>
        vert_origin_y: "vert-origin-y";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/viewBox>
        view_box: "viewBox";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/viewTarget>
        view_target: "viewTarget";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/visibility>
        // visibility: "visibility";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/width>
        // width: "width";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/widths>
        widths: "widths";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/word-spacing>
        // word_spacing: "word-spacing";

        // /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/writing-mode>
        // writing_mode: "writing-mode";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/x>
        x: "x";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/x-height>
        x_height: "x-height";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/x1>
        x1: "x1";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/x2>
        x2: "x2";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/xmlns>
        xmlns: "xmlns";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/xChannelSelector>
        x_channel_selector: "xChannelSelector";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/y>
        y: "y";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/y1>
        y1: "y1";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/y2>
        y2: "y2";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/yChannelSelector>
        y_channel_selector: "yChannelSelector";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/z>
        z: "z";

        /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/zoomAndPan>
        zoomAndPan: "zoomAndPan";
    ]
}
