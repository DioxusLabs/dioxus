use std::sync::mpsc::channel;
use std::sync::Arc;

use dioxus_core::prelude::*;
use dioxus_core::virtual_dom::VirtualDom;
use web_view::Handle;
use web_view::{WVResult, WebView, WebViewBuilder};

static HTML_CONTENT: &'static str = include_str!("./../../liveview/index.html");

pub fn launch<T: Properties + 'static>(
    builder: impl FnOnce(DioxusWebviewBuilder) -> DioxusWebviewBuilder,
    props: T,
    root: FC<T>,
) -> anyhow::Result<()> {
    WebviewRenderer::run(root, props, builder)
}

/// The `WebviewRenderer` provides a way of rendering a Dioxus Virtual DOM through a bridge to a Webview instance.
/// Components used in WebviewRenderer instances can directly use system libraries, access the filesystem, and multithread with ease.
pub struct WebviewRenderer<T> {
    /// The root component used to render the Webview
    root: FC<T>,
}
enum InnerEvent {
    Initiate(Handle<()>),
}

impl<T: Properties + 'static> WebviewRenderer<T> {
    pub fn run(
        root: FC<T>,
        props: T,
        user_builder: impl FnOnce(DioxusWebviewBuilder) -> DioxusWebviewBuilder,
    ) -> anyhow::Result<()> {
        let (sender, receiver) = channel::<InnerEvent>();

        let DioxusWebviewBuilder {
            title,
            width,
            height,
            resizable,
            debug,
            frameless,
            visible,
            min_width,
            min_height,
        } = user_builder(DioxusWebviewBuilder::new());

        let mut view = web_view::builder()
            .invoke_handler(|view, arg| {
                let handle = view.handle();
                sender
                    .send(InnerEvent::Initiate(handle))
                    .expect("should not fail");

                Ok(())
            })
            .content(web_view::Content::Html(HTML_CONTENT))
            .user_data(())
            .title(title)
            .size(width, height)
            .resizable(resizable)
            .debug(debug)
            .frameless(frameless)
            .visible(visible)
            .min_size(min_width, min_height)
            .build()
            .unwrap();

        let mut vdom = VirtualDom::new_with_props(root, props);
        let edits = vdom.rebuild()?;
        let ref_edits = Arc::new(serde_json::to_string(&edits)?);

        loop {
            view.step()
                .expect("should not fail")
                .expect("should not fail");

            if let Ok(event) = receiver.try_recv() {
                if let InnerEvent::Initiate(handle) = event {
                    let editlist = ref_edits.clone();
                    handle
                        .dispatch(move |view| {
                            view.eval(format!("EditListReceived(`{}`);", editlist).as_str())
                        })
                        .expect("Dispatch failed");
                }
            }
        }
    }

    /// Create a new text-renderer instance from a functional component root.
    /// Automatically progresses the creation of the VNode tree to completion.
    ///
    /// A VDom is automatically created. If you want more granular control of the VDom, use `from_vdom`
    pub fn new(root: FC<T>, builder: impl FnOnce() -> WVResult<WebView<'static, ()>>) -> Self {
        Self { root }
    }

    /// Create a new text renderer from an existing Virtual DOM.
    /// This will progress the existing VDom's events to completion.
    pub fn from_vdom() -> Self {
        todo!()
    }

    /// Pass new args to the root function
    pub fn update(&mut self, new_val: T) {
        todo!()
    }

    /// Modify the root function in place, forcing a re-render regardless if the props changed
    pub fn update_mut(&mut self, modifier: impl Fn(&mut T)) {
        todo!()
    }
}

pub struct DioxusWebviewBuilder<'a> {
    pub(crate) title: &'a str,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) resizable: bool,
    pub(crate) debug: bool,
    pub(crate) frameless: bool,
    pub(crate) visible: bool,
    pub(crate) min_width: i32,
    pub(crate) min_height: i32,
}
impl<'a> DioxusWebviewBuilder<'a> {
    fn new() -> Self {
        #[cfg(debug_assertions)]
        let debug = true;
        #[cfg(not(debug_assertions))]
        let debug = false;

        DioxusWebviewBuilder {
            title: "Application",
            width: 800,
            height: 600,
            resizable: true,
            debug,
            frameless: false,
            visible: true,
            min_width: 300,
            min_height: 300,
        }
    }
    /// Sets the title of the WebView window.
    ///
    /// Defaults to `"Application"`.
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    /// Sets the size of the WebView window.
    ///
    /// Defaults to 800 x 600.
    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Sets the resizability of the WebView window. If set to false, the window cannot be resized.
    ///
    /// Defaults to `true`.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Enables or disables debug mode.
    ///
    /// Defaults to `true` for debug builds, `false` for release builds.
    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }
    /// The window crated will be frameless
    ///
    /// defaults to `false`
    pub fn frameless(mut self, frameless: bool) -> Self {
        self.frameless = frameless;
        self
    }

    /// Set the visibility of the WebView window.
    ///
    /// defaults to `true`
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Sets the minimum size of the WebView window.
    ///
    /// Defaults to 300 x 300.
    pub fn min_size(mut self, width: i32, height: i32) -> Self {
        self.min_width = width;
        self.min_height = height;
        self
    }
}
