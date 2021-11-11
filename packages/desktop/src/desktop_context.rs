use std::cell::RefCell;

use dioxus::prelude::Scope;
use dioxus_core as dioxus;
use dioxus_core::{Context, Element, LazyNodes, NodeFactory, Properties};
use dioxus_core_macro::Props;

/*
This module provides a set of Dioxus components to easily manage windows, tabs, etc.

Windows can be created anywhere in the tree, making them very flexible for things like modals, etc.

*/
pub struct DesktopContext {}

impl DesktopContext {
    fn add_window(&mut self) {
        //
    }
    fn close_window(&mut self) {
        //
    }
}

enum WindowHandlers {
    Resized(Box<dyn Fn()>),
    Moved(Box<dyn Fn()>),
    CloseRequested(Box<dyn Fn()>),
    Destroyed(Box<dyn Fn()>),
    DroppedFile(Box<dyn Fn()>),
    HoveredFile(Box<dyn Fn()>),
    HoverFileCancelled(Box<dyn Fn()>),
    ReceivedTimeText(Box<dyn Fn()>),
    Focused(Box<dyn Fn()>),
}

#[derive(Props)]
pub struct WebviewWindowProps<'a> {
    onclose: &'a dyn FnMut(()),

    onopen: &'a dyn FnMut(()),

    /// focuse me
    onfocused: &'a dyn FnMut(()),

    children: Element,
}

/// A handle to a
///
///
///
///
///
///
///
///
///
pub fn WebviewWindow(cx: Context, props: &WebviewWindowProps) -> Element {
    let dtcx = cx.consume_state::<RefCell<DesktopContext>>()?;

    cx.use_hook(
        |_| {
            //
        },
        |state| {
            //
        },
    );

    // render the children directly
    todo!()
    // cx.render(LazyNodes::new(move |f: NodeFactory| {
    //     f.fragment_from_iter(cx.children())
    // }))
}

pub struct WindowHandle {}

/// Get a handle to the current window from inside a component
pub fn use_current_window(cx: Context) -> Option<WindowHandle> {
    todo!()
}

#[test]
fn syntax_works() {
    use dioxus_core as dioxus;
    use dioxus_core::prelude::*;
    use dioxus_core_macro::*;
    use dioxus_hooks::*;
    use dioxus_html as dioxus_elements;

    static App: FC<()> = |cx, props| {
        cx.render(rsx! {
            // left window
            WebviewWindow {
                onclose: move |evt| {}
                onopen: move |evt| {}
                onfocused: move |evt| {}

                div {

                }
            }
        })
    };
}
