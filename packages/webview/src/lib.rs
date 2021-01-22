use dioxus_core::prelude::*;
use web_view::WebViewBuilder;

pub fn new<T>(root: FC<T>) -> WebviewRenderer<T> {
    WebviewRenderer::new(root)
}

/// The `WebviewRenderer` provides a way of rendering a Dioxus Virtual DOM through a bridge to a Webview instance.
/// Components used in WebviewRenderer instances can directly use system libraries, access the filesystem, and multithread with ease.
pub struct WebviewRenderer<T> {
    /// The root component used to render the Webview
    root: FC<T>,
}

impl<T> WebviewRenderer<T> {
    /// Create a new text-renderer instance from a functional component root.
    /// Automatically progresses the creation of the VNode tree to completion.
    ///
    /// A VDom is automatically created. If you want more granular control of the VDom, use `from_vdom`
    pub fn new(root: FC<T>) -> Self {
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

    pub fn launch(self, props: T) {
        let mut ctx = Context { props: &props };
        let WebviewRenderer { root } = self;
        let content = root(&mut ctx);
        let html_content = content.to_string();
        /*
        TODO: @Jon
        Launch the webview with a premade VDom app
        */

        web_view::builder()
            .title("My Project")
            .content(web_view::Content::Html(html_content))
            .size(320, 480)
            .resizable(true)
            .debug(true)
            .user_data(())
            .invoke_handler(|_webview, _arg| Ok(()))
            .run()
            .unwrap();
    }
}

mod receiver {
    use dioxus_core::prelude::*;

    /// The receiver app is a container that builds a connection to the host process that shuttles events and patches.  
    pub(crate) static ReceiverApp: FC<()> = |ctx| {
        //
        html! {
            <div>
                {}
            </div>
        }
    };
}
