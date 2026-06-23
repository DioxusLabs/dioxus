// Organized in the same order as
// https://developer.mozilla.org/en-US/docs/Web/HTML/Element
//
// Does not include obsolete elements.
//
// This namespace represents a collection of modern HTML-5 compatible elements.
//
// This list does not include obsolete, deprecated, experimental, or poorly supported elements.

// Re-export the element-vocabulary root so that any `use dioxus_html::elements::*` (the common
// way to bring elements into scope) also brings `html` into scope. `rsx!` lowers a bare element
// like `div` to the associated const `html::div`, which needs the root type reachable here.
pub use crate::html;

dioxus_html_internal_macro::define_elements! {
    core = dioxus_core,
    html = crate,
    context = true;

    // Document metadata

    /// Build a
    /// [`<base>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/base)
    /// element.
    ///
    base {
        href,
        target,
    }

    /// Build a
    /// [`<head>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/head)
    /// element.
    head {}

    /// Build a
    /// [`<link>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link)
    /// element.
    link {
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
    }

    /// Build a
    /// [`<meta>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta)
    /// element.
    meta {
        charset, // FIXME IANA standard names
        content,
        #[attr(name = "http-equiv")]
        http_equiv,
        name,
        property,
    }

    /// Build a
    /// [`<style>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/style)
    /// element.
    style {
        #[attr(name = "type")]
        r#type,
        media, // FIXME media query
        nonce,
        title, // FIXME
    }

    /// Build a
    /// [`<title>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/title)
    /// element.
    title {}

    // Sectioning root

    /// Build a
    /// [`<body>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/body)
    /// element.
    body {}

    // ------------------
    // Content sectioning
    // ------------------

    /// Build a
    /// [`<address>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/address)
    /// element.
    address {}

    /// Build a
    /// [`<article>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/article)
    /// element.
    article {}

    /// Build a
    /// [`<aside>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/aside)
    /// element.
    aside {}

    /// Build a
    /// [`<footer>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/footer)
    /// element.
    footer {}

    /// Build a
    /// [`<header>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/header)
    /// element.
    header {}

    /// Build a
    /// [`<hgroup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/hgroup)
    /// element.
    hgroup {}

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
    h1 {}

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
    h2 {}

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
    h3 {}

    /// Build a
    /// [`<h4>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h4)
    /// element.
    h4 {}

    /// Build a
    /// [`<h5>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h5)
    /// element.
    h5 {}

    /// Build a
    /// [`<h6>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h6)
    /// element.
    h6 {}

    /// Build a
    /// [`<main>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/main)
    /// element.
    main {}

    /// Build a
    /// [`<nav>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/nav)
    /// element.
    nav {}

    /// Build a
    /// [`<section>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/section)
    /// element.
    section {}

    // Text content

    /// Build a
    /// [`<blockquote>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/blockquote)
    /// element.
    blockquote {
        cite,
    }
    /// Build a
    /// [`<dd>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dd)
    /// element.
    dd {}

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
    div {}

    /// Build a
    /// [`<dl>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dl)
    /// element.
    dl {}

    /// Build a
    /// [`<dt>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dt)
    /// element.
    dt {}

    /// Build a
    /// [`<figcaption>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/figcaption)
    /// element.
    figcaption {}

    /// Build a
    /// [`<figure>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/figure)
    /// element.
    figure {}

    /// Build a
    /// [`<hr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/hr)
    /// element.
    hr {}

    /// Build a
    /// [`<li>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/li)
    /// element.
    li {
        value,
    }

    /// Build a
    /// [`<ol>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ol)
    /// element.
    ol {
        reversed,
        start,
        #[attr(name = "type")]
        r#type,
    }

    /// Build a
    /// [`<p>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/p)
    /// element.
    p {}

    /// Build a
    /// [`<pre>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/pre)
    /// element.
    pre {}

    /// Build a
    /// [`<ul>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ul)
    /// element.
    ul {}


    // Inline text semantics

    /// Build a
    /// [`<a>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/a)
    /// element.
    a {
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
    }

    /// Build a
    /// [`<abbr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/abbr)
    /// element.
    abbr {}

    /// Build a
    /// [`<b>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/b)
    /// element.
    b {}

    /// Build a
    /// [`<bdi>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/bdi)
    /// element.
    bdi {}

    /// Build a
    /// [`<bdo>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/bdo)
    /// element.
    bdo {}

    /// Build a
    /// [`<br>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/br)
    /// element.
    br {}

    /// Build a
    /// [`<cite>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/cite)
    /// element.
    cite {}

    /// Build a
    /// [`<code>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/code)
    /// element.
    code {
        language,
    }

    /// Build a
    /// [`<data>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/data)
    /// element.
    data {
        value,
    }

    /// Build a
    /// [`<dfn>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dfn)
    /// element.
    dfn {}

    /// Build a
    /// [`<em>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/em)
    /// element.
    em {}

    /// Build a
    /// [`<i>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/i)
    /// element.
    i {}

    /// Build a
    /// [`<kbd>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/kbd)
    /// element.
    kbd {}

    /// Build a
    /// [`<mark>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/mark)
    /// element.
    mark {}

    /// Build a
    /// [`<menu>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/menu)
    /// element.
    menu {}

    /// Build a
    /// [`<q>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/q)
    /// element.
    q {
        cite,
    }


    /// Build a
    /// [`<rp>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/rp)
    /// element.
    rp {}


    /// Build a
    /// [`<rt>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/rt)
    /// element.
    rt {}


    /// Build a
    /// [`<ruby>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ruby)
    /// element.
    ruby {}

    /// Build a
    /// [`<s>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/s)
    /// element.
    s {}

    /// Build a
    /// [`<samp>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/samp)
    /// element.
    samp {}

    /// Build a
    /// [`<small>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/small)
    /// element.
    small {}

    /// Build a
    /// [`<span>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/span)
    /// element.
    span {}

    /// Build a
    /// [`<strong>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/strong)
    /// element.
    strong {}

    /// Build a
    /// [`<sub>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/sub)
    /// element.
    sub {}

    /// Build a
    /// [`<sup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/sup)
    /// element.
    sup {}

    /// Build a
    /// [`<time>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/time)
    /// element.
    time {
        datetime,
    }

    /// Build a
    /// [`<u>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/u)
    /// element.
    u {}

    /// Build a
    /// [`<var>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/var)
    /// element.
    var {}

    /// Build a
    /// [`<wbr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/wbr)
    /// element.
    wbr {}


    // Image and multimedia

    /// Build a
    /// [`<area>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/area)
    /// element.
    area {
        alt,
        coords, // TODO could perhaps be validated
        download,
        href,
        hreflang,
        shape,
        target,
        // ping: SpacedList<Uri>,
        // rel: SpacedSet<LinkType>,
    }

    /// Build a
    /// [`<audio>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/audio)
    /// element.
    audio {
        autoplay,
        controls,
        crossorigin,
        muted,
        preload,
        src,
        #[attr(name = "loop")]
        r#loop,
    }

    /// Build a
    /// [`<img>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/img)
    /// element.
    img {
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
    }

    /// Build a
    /// [`<map>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/map)
    /// element.
    map {
        name,
    }

    /// Build a
    /// [`<track>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/track)
    /// element.
    track {
        default,
        kind,
        label,
        src,
        srclang,
    }

    /// Build a
    /// [`<video>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/video)
    /// element.
    video {
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
    }


    // Embedded content

    /// Build a
    /// [`<embed>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/embed)
    /// element.
    embed {
        height,
        src,
        #[attr(name = "type")]
        r#type,
        width,
    }

    /// Build a
    /// [`<iframe>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/iframe)
    /// element.
    iframe {
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
    }

    /// Build a
    /// [`<object>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/object)
    /// element.
    object {
        data,
        form,
        height,
        name,
        #[attr(name = "type")]
        r#type,
        typemustmatch,
        usemap, // TODO should be a fragment starting with '#'
        width,
    }

    /// Build a
    /// [`<param>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/param)
    /// element.
    param {
        name,
        value,
    }

    /// Build a
    /// [`<picture>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/picture)
    /// element.
    picture {}

    /// Build a
    /// [`<source>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/source)
    /// element.
    source {
        src,
        #[attr(name = "type")]
        r#type,
        srcset,
        media,
        sizes,
        width,
        height,
    }


    // Scripting

    /// Build a
    /// [`<canvas>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/canvas)
    /// element.
    canvas {
        height,
        width,
    }

    /// Build a
    /// [`<noscript>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/noscript)
    /// element.
    noscript {}

    /// Build a
    /// [`<script>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script)
    /// element.
    ///
    /// The script HTML element is used to embed executable code or data; this is typically used to embed or refer to
    /// JavaScript code. The script element can also be used with other languages, such as WebGL's GLSL shader
    /// programming language and JSON.
    script {
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
    }


    // Demarcating edits

    /// Build a
    /// [`<del>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/del)
    /// element.
    del {
        cite,
        datetime,
    }

    /// Build a
    /// [`<ins>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ins)
    /// element.
    ins {
        cite,
        datetime,
    }


    // Table content

    /// Build a
    /// [`<caption>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/caption)
    /// element.
    caption {}

    /// Build a
    /// [`<col>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/col)
    /// element.
    col {
        span,
    }

    /// Build a
    /// [`<colgroup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/colgroup)
    /// element.
    colgroup {
        span,
    }

    /// Build a
    /// [`<table>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/table)
    /// element.
    table {}

    /// Build a
    /// [`<tbody>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tbody)
    /// element.
    tbody {}

    /// Build a
    /// [`<td>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/td)
    /// element.
    td {
        colspan,
        rowspan,
        // headers: SpacedSet<Id>,
    }

    /// Build a
    /// [`<tfoot>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tfoot)
    /// element.
    tfoot {}

    /// Build a
    /// [`<th>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/th)
    /// element.
    th {
        abbr,
        colspan,
        rowspan,
        scope,
        // headers: SpacedSet<Id>,
    }

    /// Build a
    /// [`<thead>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/thead)
    /// element.
    thead {}

    /// Build a
    /// [`<tr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tr)
    /// element.
    tr {}


    // Forms

    /// Build a
    /// [`<button>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/button)
    /// element.
    button {
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
    }

    /// Build a
    /// [`<datalist>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/datalist)
    /// element.
    datalist {}

    /// Build a
    /// [`<fieldset>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/fieldset)
    /// element.
    fieldset {
        disabled,
        form,
        name,
    }

    /// Build a
    /// [`<form>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
    /// element.
    form {
        // accept-charset: SpacedList<CharacterEncoding>,
        action,
        autocomplete,
        enctype,
        method,
        name,
        novalidate,
        target,
    }

    /// Build a
    /// [`<input>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input)
    /// element.
    input {
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
    }

    /// Build a
    /// [`<label>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/label)
    /// element.
    label {
        form,
        #[attr(name = "for")]
        r#for,
    }

    /// Build a
    /// [`<legend>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/legend)
    /// element.
    legend {}

    /// Build a
    /// [`<meter>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meter)
    /// element.
    meter {
        value,
        min,
        max,
        low,
        high,
        optimum,
        form,
    }

    /// Build a
    /// [`<optgroup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/optgroup)
    /// element.
    optgroup {
        disabled,
        label,
    }

    /// Build a
    /// [`<option>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/option)
    /// element.
    option {
        disabled,
        label,


        value,

        #[attr(volatile)]
        selected,
        initial_selected,
    }

    /// Build a
    /// [`<output>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/output)
    /// element.
    output {
        form,
        name,
        // r#for: SpacedSet<Id>,
    }

    /// Build a
    /// [`<progress>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/progress)
    /// element.
    progress {
        max,
        value,
    }

    /// Build a
    /// [`<select>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/select)
    /// element.
    select {
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
    }

    /// Build a
    /// [`<textarea>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/textarea)
    /// element.
    textarea {
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
    }


    // Interactive elements

    /// Build a
    /// [`<details>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/details)
    /// element.
    details {
        open,
    }

    /// Build dialog
    /// [`<dialog>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dialog)
    /// element.
    dialog {
        open,
    }

    /// Build a
    /// [`<summary>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/summary)
    /// element.
    summary {}

    // Web components

    /// Build a
    /// [`<slot>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/slot)
    /// element.
    slot {
        name,
    }

    /// Build a
    /// [`<template>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/template)
    /// element.
    template {}

    // SVG components
    /// Build a
    /// [`<svg>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/svg)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    svg {}


    // /// Build a
    // /// [`<a>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/a)
    // /// element.
    // #[element(namespace = "http://www.w3.org/2000/svg")]
    // a {}

    /// Build a
    /// [`<animate>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/animate)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    animate {}

    /// Build a
    /// [`<animateMotion>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/animateMotion)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    animateMotion {}

    /// Build a
    /// [`<animateTransform>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/animateTransform)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    animateTransform {}

    /// Build a
    /// [`<circle>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/circle)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    circle {}

    /// Build a
    /// [`<clipPath>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/clipPath)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    clipPath {}

    /// Build a
    /// [`<defs>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/defs)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    defs {}

    /// Build a
    /// [`<desc>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/desc)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    desc {}

    /// Build a
    /// [`<discard>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/discard)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    discard {}

    /// Build a
    /// [`<ellipse>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/ellipse)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    ellipse {}

    /// Build a
    /// [`<feBlend>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feBlend)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feBlend {}

    /// Build a
    /// [`<feColorMatrix>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feColorMatrix)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feColorMatrix {}

    /// Build a
    /// [`<feComponentTransfer>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feComponentTransfer)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feComponentTransfer {}

    /// Build a
    /// [`<feComposite>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feComposite)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feComposite {}

    /// Build a
    /// [`<feConvolveMatrix>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feConvolveMatrix)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feConvolveMatrix {}

    /// Build a
    /// [`<feDiffuseLighting>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feDiffuseLighting)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feDiffuseLighting {}

    /// Build a
    /// [`<feDisplacementMap>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feDisplacementMap)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feDisplacementMap {}

    /// Build a
    /// [`<feDistantLight>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feDistantLight)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feDistantLight {}

    /// Build a
    /// [`<feDropShadow>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feDropShadow)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feDropShadow {}

    /// Build a
    /// [`<feFlood>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feFlood)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feFlood {}

    /// Build a
    /// [`<feFuncA>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feFuncA)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feFuncA {}

    /// Build a
    /// [`<feFuncB>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feFuncB)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feFuncB {}

    /// Build a
    /// [`<feFuncG>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feFuncG)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feFuncG {}

    /// Build a
    /// [`<feFuncR>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feFuncR)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feFuncR {}

    /// Build a
    /// [`<feGaussianBlur>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feGaussianBlur)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feGaussianBlur {}

    /// Build a
    /// [`<feImage>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feImage)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feImage {}

    /// Build a
    /// [`<feMerge>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feMerge)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feMerge {}

    /// Build a
    /// [`<feMergeNode>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feMergeNode)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feMergeNode {}

    /// Build a
    /// [`<feMorphology>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feMorphology)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feMorphology {}

    /// Build a
    /// [`<feOffset>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feOffset)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feOffset {}

    /// Build a
    /// [`<fePointLight>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/fePointLight)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    fePointLight {}

    /// Build a
    /// [`<feSpecularLighting>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feSpecularLighting)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feSpecularLighting {}

    /// Build a
    /// [`<feSpotLight>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feSpotLight)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feSpotLight {}

    /// Build a
    /// [`<feTile>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feTile)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feTile {}

    /// Build a
    /// [`<feTurbulence>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feTurbulence)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    feTurbulence {}

    /// Build a
    /// [`<filter>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/filter)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    filter {}

    /// Build a
    /// [`<foreignObject>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/foreignObject)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    foreignObject {}

    /// Build a
    /// [`<g>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/g)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    g {}

    /// Build a
    /// [`<hatch>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/hatch)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    hatch {}

    /// Build a
    /// [`<hatchpath>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/hatchpath)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    hatchpath {}

    /// Build a
    /// [`<image>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/image)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    image {}

    /// Build a
    /// [`<line>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/line)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    line {}

    /// Build a
    /// [`<linearGradient>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/linearGradient)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    linearGradient {}

    /// Build a
    /// [`<marker>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/marker)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    marker {}

    /// Build a
    /// [`<mask>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/mask)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    mask {}

    /// Build a
    /// [`<metadata>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/metadata)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    metadata {}

    /// Build a
    /// [`<mpath>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/mpath)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    mpath {}

    /// Build a
    /// [`<path>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/path)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    path {}

    /// Build a
    /// [`<pattern>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/pattern)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    pattern {}

    /// Build a
    /// [`<polygon>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/polygon)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    polygon {}

    /// Build a
    /// [`<polyline>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/polyline)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    polyline {}

    /// Build a
    /// [`<radialGradient>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/radialGradient)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    radialGradient {}

    /// Build a
    /// [`<rect>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/rect)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    rect {}

    // /// Build a
    // /// [`<script>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/script)
    // /// element.
    // #[element(namespace = "http://www.w3.org/2000/svg")]
    // script {}

    /// Build a
    /// [`<set>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/set)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    set {}

    /// Build a
    /// [`<stop>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/stop)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    stop {}

    // /// Build a
    // /// [`<style>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/style)
    // /// element.
    // #[element(namespace = "http://www.w3.org/2000/svg")]
    // style {}

    // /// Build a
    // /// [`<svg>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/svg)
    // /// element.
    // #[element(namespace = "http://www.w3.org/2000/svg")]
    // svg {}

    /// Build a
    /// [`<switch>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/switch)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    switch {}

    /// Build a
    /// [`<symbol>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/symbol)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    symbol {}

    /// Build a
    /// [`<text>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/text)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    text {}

    /// Build a
    /// [`<textPath>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/textPath)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    textPath {}

    // /// Build a
    // /// [`<title>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/title)
    // /// element.
    // #[element(namespace = "http://www.w3.org/2000/svg")]
    // title {}

    /// Build a
    /// [`<tspan>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/tspan)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    tspan {}

    /// Build a
    /// [`<view>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/view)
    /// element.
    #[element(namespace = "http://www.w3.org/2000/svg")]
    view {}

    // /// Build a
    // /// [`<use>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/use)
    // /// element.
    #[element(name = "use", namespace = "http://www.w3.org/2000/svg")]
    r#use {
        href,
    }

    // MathML elements

    /// Build a
    /// [`<annotation>`](https://w3c.github.io/mathml-core/#dfn-annotation)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    annotation {
            encoding,
    }

    /// Build a
    /// [`<annotation-xml>`](https://w3c.github.io/mathml-core/#dfn-annotation-xml)
    /// element.
    #[element(name = "annotation-xml", namespace = "http://www.w3.org/1998/Math/MathML")]
    annotationXml {
            encoding,
    }

    /// Build a
    /// [`<merror>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/merror)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    merror {}

    /// Build a
    /// [`<math>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/math)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    math {
        display,
    }

    /// Build a
    /// [`<mfrac>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mfrac)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    mfrac {
        linethickness,
    }

    /// Build a
    /// [`<mi>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mi)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    mi {
        mathvariant,
    }

    /// Build a
    /// [`<mmultiscripts>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mmultiscripts)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    mmultiscripts {}

    /// Build a
    /// [`<mn>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mn)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    mn {}

    /// Build a
    /// [`<mo>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mo)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    mo {
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
    }

    /// Build a
    /// [`<mover>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mover)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    mover {
        accent,
    }

    /// Build a
    /// [`<mpadded>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mpadded)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    mpadded {
        depth,
        height,
        lspace,
        voffset,
        width,
    }

    /// Build a
    /// [`<mphantom>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mphantom)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    mphantom {}

    /// Build a
    /// [`<mprescripts>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mprescripts)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    mprescripts {}

    /// Build a
    /// [`<mroot>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mroot)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    mroot {}

    /// Build a
    /// [`<mrow>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mrow)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    mrow {

    }

    /// Build a
    /// [`<ms>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/ms)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    ms {
        lquote,
        rquote,
    }

    /// Build a
    /// [`<mspace>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mspace)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    mspace {
        depth,
        height,
        width,
    }

    /// Build a
    /// [`<msqrt>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/msqrt)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    msqrt {}

    /// Build a
    /// [`<mstyle>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mstyle)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    mstyle {}

    /// Build a
    /// [`<msub>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/msub)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    msub {}

    /// Build a
    /// [`<msubsup>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/msubsup)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    msubsup {}

    /// Build a
    /// [`<msup>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/msup)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    msup {}

    /// Build a
    /// [`<mtable>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mtable)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    mtable {}

    /// Build a
    /// [`<mtd>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mtd)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    mtd {
        columnspan,
        rowspan,
    }

    /// Build a
    /// [`<mtext>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mtext)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    mtext {}

    /// Build a
    /// [`<mtr>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/mtr)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    mtr {}

    /// Build a
    /// [`<munder>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/munder)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    munder {
        accentunder,
    }

    /// Build a
    /// [`<munderover>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/munderover)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    munderover {
        accent,
        accentunder,
    }

    /// Build a
    /// [`<semantics>`](https://developer.mozilla.org/en-US/docs/Web/MathML/Element/semantics)
    /// element.
    #[element(namespace = "http://www.w3.org/1998/Math/MathML")]
    semantics {
        encoding,
    }
}
