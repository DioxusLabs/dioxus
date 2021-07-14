//! Dioxus WebSys
//! --------------
//! This crate implements a renderer of the Dioxus Virtual DOM for the web browser using Websys.

use dioxus::prelude::{Context, Properties, VNode};
use futures_util::{pin_mut, Stream, StreamExt};

use fxhash::FxHashMap;
use web_sys::{window, Document, Element, Event, Node};
// use futures::{channel::mpsc, SinkExt, StreamExt};

use dioxus::virtual_dom::VirtualDom;
pub use dioxus_core as dioxus;
use dioxus_core::{events::EventTrigger, prelude::FC};

pub use dioxus_core::prelude;
// pub mod interpreter;
pub mod new;

/// The `WebsysRenderer` provides a way of rendering a Dioxus Virtual DOM to the browser's DOM.
/// Under the hood, we leverage WebSys and interact directly with the DOM
///
pub struct WebsysRenderer {
    internal_dom: VirtualDom,
}

impl WebsysRenderer {
    /// This method is the primary entrypoint for Websys Dioxus apps. Will panic if an error occurs while rendering.
    /// See DioxusErrors for more information on how these errors could occour.
    ///
    /// ```ignore
    /// fn main() {
    ///     wasm_bindgen_futures::spawn_local(WebsysRenderer::start(Example));
    /// }
    /// ```
    ///
    /// Run the app to completion, panicing if any error occurs while rendering.
    /// Pairs well with the wasm_bindgen async handler
    pub async fn start(root: FC<()>) {
        Self::new(root).run().await.expect("Virtual DOM failed :(");
    }

    /// Create a new instance of the Dioxus Virtual Dom with no properties for the root component.
    ///
    /// This means that the root component must either consumes its own context, or statics are used to generate the page.
    /// The root component can access things like routing in its context.
    pub fn new(root: FC<()>) -> Self {
        Self::new_with_props(root, ())
    }

    /// Create a new text-renderer instance from a functional component root.
    /// Automatically progresses the creation of the VNode tree to completion.
    ///
    /// A VDom is automatically created. If you want more granular control of the VDom, use `from_vdom`
    pub fn new_with_props<T: Properties + 'static>(root: FC<T>, root_props: T) -> Self {
        Self::from_vdom(VirtualDom::new_with_props(root, root_props))
    }

    /// Create a new text renderer from an existing Virtual DOM.
    pub fn from_vdom(dom: VirtualDom) -> Self {
        Self { internal_dom: dom }
    }

    pub async fn run(&mut self) -> dioxus_core::error::Result<()> {
        use wasm_bindgen::JsCast;

        let root = prepare_websys_dom();
        let root_node = root.clone().dyn_into::<Node>().unwrap();

        let mut websys_dom = crate::new::WebsysDom::new(root.clone());

        websys_dom.stack.push(root_node.clone());
        websys_dom.stack.push(root_node);

        self.internal_dom.rebuild(&mut websys_dom)?;

        log::info!("Going into event loop");
        // loop {
        let trigger = {
            let real_queue = websys_dom.wait_for_event();
            if self.internal_dom.tasks.is_empty() {
                log::info!("tasks is empty, waiting for dom event to trigger soemthing");
                real_queue.await
            } else {
                log::info!("tasks is not empty, waiting for either tasks or event system");
                let task_queue = (&mut self.internal_dom.tasks).next();

                pin_mut!(real_queue);
                pin_mut!(task_queue);

                match futures_util::future::select(real_queue, task_queue).await {
                    futures_util::future::Either::Left((trigger, _)) => trigger,
                    futures_util::future::Either::Right((trigger, _)) => trigger,
                }
            }
        };

        if let Some(real_trigger) = trigger {
            log::info!("event received");
            // let root_node = body_element.first_child().unwrap();
            // websys_dom.stack.push(root_node.clone());

            self.internal_dom.queue_event(real_trigger)?;

            self.internal_dom
                .progress_with_event(&mut websys_dom)
                .await?;
        }

        // let t2 = self.internal_dom.tasks.next();
        // futures::select! {
        //     trigger = t1 => {
        //         log::info!("event received");
        //         let root_node = body_element.first_child().unwrap();
        //         websys_dom.stack.push(root_node.clone());
        //         self.internal_dom
        //             .progress_with_event(&mut websys_dom, trigger)?;
        //     },
        //     () = t2 => {}
        // };
        // }
        // while let Some(trigger) = websys_dom.wait_for_event().await {
        // }

        Ok(()) // should actually never return from this, should be an error, rustc just cant see it
    }
}

fn prepare_websys_dom() -> Element {
    // Initialize the container on the dom
    // Hook up the body as the root component to render tinto
    let window = web_sys::window().expect("should have access to the Window");
    let document = window
        .document()
        .expect("should have access to the Document");

    // let body = document.body().unwrap();
    let el = document.get_element_by_id("dioxusroot").unwrap();

    // Build a dummy div
    // let container: &Element = body.as_ref();
    // container.set_inner_html("");
    // container
    //     .append_child(
    //         document
    //             .create_element("div")
    //             .expect("should create element OK")
    //             .as_ref(),
    //     )
    //     .expect("should append child OK");
    el
    // container.clone()
}

// Progress the mount of the root component

// Iterate through the nodes, attaching the closure and sender to the listener
// {
//     let mut remote_sender = sender.clone();
//     let listener = move || {
//         let event = EventTrigger::new();
//         wasm_bindgen_futures::spawn_local(async move {
//             remote_sender
//                 .send(event)
//                 .await
//                 .expect("Updating receiver failed");
//         })
//     };
// }

/// Wasm-bindgen has a performance option to intern commonly used phrases
/// This saves the decoding cost, making the interaction of Rust<->JS more performant.
/// We intern all the HTML tags and attributes, making most operations much faster.
///
/// Interning takes about 1ms at the start of the app, but saves a *ton* of time later on.
pub fn intern_cache() {
    let cached_words = [
        // All the HTML Tags
        "a",
        "abbr",
        "address",
        "area",
        "article",
        "aside",
        "audio",
        "b",
        "base",
        "bdi",
        "bdo",
        "big",
        "blockquote",
        "body",
        "br",
        "button",
        "canvas",
        "caption",
        "cite",
        "code",
        "col",
        "colgroup",
        "command",
        "data",
        "datalist",
        "dd",
        "del",
        "details",
        "dfn",
        "dialog",
        "div",
        "dl",
        "dt",
        "em",
        "embed",
        "fieldset",
        "figcaption",
        "figure",
        "footer",
        "form",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "head",
        "header",
        "hr",
        "html",
        "i",
        "iframe",
        "img",
        "input",
        "ins",
        "kbd",
        "keygen",
        "label",
        "legend",
        "li",
        "link",
        "main",
        "map",
        "mark",
        "menu",
        "menuitem",
        "meta",
        "meter",
        "nav",
        "noscript",
        "object",
        "ol",
        "optgroup",
        "option",
        "output",
        "p",
        "param",
        "picture",
        "pre",
        "progress",
        "q",
        "rp",
        "rt",
        "ruby",
        "s",
        "samp",
        "script",
        "section",
        "select",
        "small",
        "source",
        "span",
        "strong",
        "style",
        "sub",
        "summary",
        "sup",
        "table",
        "tbody",
        "td",
        "textarea",
        "tfoot",
        "th",
        "thead",
        "time",
        "title",
        "tr",
        "track",
        "u",
        "ul",
        "var",
        "video",
        "wbr",
        // All the event handlers
        "Attribute",
        "accept",
        "accept-charset",
        "accesskey",
        "action",
        "alt",
        "async",
        "autocomplete",
        "autofocus",
        "autoplay",
        "charset",
        "checked",
        "cite",
        "class",
        "cols",
        "colspan",
        "content",
        "contenteditable",
        "controls",
        "coords",
        "data",
        "data-*",
        "datetime",
        "default",
        "defer",
        "dir",
        "dirname",
        "disabled",
        "download",
        "draggable",
        "enctype",
        "for",
        "form",
        "formaction",
        "headers",
        "height",
        "hidden",
        "high",
        "href",
        "hreflang",
        "http-equiv",
        "id",
        "ismap",
        "kind",
        "label",
        "lang",
        "list",
        "loop",
        "low",
        "max",
        "maxlength",
        "media",
        "method",
        "min",
        "multiple",
        "muted",
        "name",
        "novalidate",
        "onabort",
        "onafterprint",
        "onbeforeprint",
        "onbeforeunload",
        "onblur",
        "oncanplay",
        "oncanplaythrough",
        "onchange",
        "onclick",
        "oncontextmenu",
        "oncopy",
        "oncuechange",
        "oncut",
        "ondblclick",
        "ondrag",
        "ondragend",
        "ondragenter",
        "ondragleave",
        "ondragover",
        "ondragstart",
        "ondrop",
        "ondurationchange",
        "onemptied",
        "onended",
        "onerror",
        "onfocus",
        "onhashchange",
        "oninput",
        "oninvalid",
        "onkeydown",
        "onkeypress",
        "onkeyup",
        "onload",
        "onloadeddata",
        "onloadedmetadata",
        "onloadstart",
        "onmousedown",
        "onmousemove",
        "onmouseout",
        "onmouseover",
        "onmouseup",
        "onmousewheel",
        "onoffline",
        "ononline",
        "<body>",
        "onpageshow",
        "onpaste",
        "onpause",
        "onplay",
        "onplaying",
        "<body>",
        "onprogress",
        "onratechange",
        "onreset",
        "onresize",
        "onscroll",
        "onsearch",
        "onseeked",
        "onseeking",
        "onselect",
        "onstalled",
        "<body>",
        "onsubmit",
        "onsuspend",
        "ontimeupdate",
        "ontoggle",
        "onunload",
        "onvolumechange",
        "onwaiting",
        "onwheel",
        "open",
        "optimum",
        "pattern",
        "placeholder",
        "poster",
        "preload",
        "readonly",
        "rel",
        "required",
        "reversed",
        "rows",
        "rowspan",
        "sandbox",
        "scope",
        "selected",
        "shape",
        "size",
        "sizes",
        "span",
        "spellcheck",
        "src",
        "srcdoc",
        "srclang",
        "srcset",
        "start",
        "step",
        "style",
        "tabindex",
        "target",
        "title",
        "translate",
        "type",
        "usemap",
        "value",
        "width",
        "wrap",
    ];

    for s in cached_words {
        wasm_bindgen::intern(s);
    }
}
