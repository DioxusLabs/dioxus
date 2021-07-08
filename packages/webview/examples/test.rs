fn main() {
    dioxus_webview::launch(App, |f| f.with_focus().with_maximized(true)).expect("Failed");
}

static App: FC<()> = |cx| {
    //
    cx.render(rsx!(
        div {
            "hello world!"
        }
    ))
};

use dioxus_core as dioxus;
use dioxus_core::prelude::*;
mod dioxus_elements {
    use super::*;
    pub struct div;
    impl DioxusElement for div {
        const TAG_NAME: &'static str = "div";
        const NAME_SPACE: Option<&'static str> = None;
    }
}
