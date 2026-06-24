//! Multiwindow example
//!
//! This example shows how to implement a simple multiwindow application using dioxus.
//! The app root owns each native window explicitly.

use dioxus::desktop::{Config, WindowBuilder, WindowConfig};
use dioxus::prelude::*;

fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(Config::new().with_headless_root(true))
        .launch(app);
}

fn app() -> Element {
    let mut window_state = use_store(|| WindowState {
        windows: vec![0],
        next_id: 1,
        count: 0,
    });

    rsx! {
        for id in window_state.windows().read().iter().copied() {
            Window {
                key: "{id}",
                config: WindowConfig::new().with_window(
                    WindowBuilder::new().with_title(format!("Window {id}"))
                ),
                onclose: move |_| window_state.close_window(id),
                AppWindow { id, window_state }
            }
        }
    }
}

#[derive(Store, PartialEq, Clone, Debug)]
struct WindowState {
    windows: Vec<usize>,
    next_id: usize,
    count: usize,
}

#[store]
impl Store<WindowState> {
    fn open_window(&mut self) {
        let id = self.next_id().cloned();
        self.next_id().set(id + 1);
        self.windows().push(id);
    }

    fn close_window(&mut self, id: usize) {
        self.windows().retain(|window_id| *window_id != id);
    }

    fn increment_count(&mut self) {
        let count = self.count().cloned();
        self.count().set(count + 1);
    }
}

#[component]
fn AppWindow(id: usize, mut window_state: Store<WindowState>) -> Element {
    rsx! {
        div {
            h1 { "Window {id}" }
            p { "Count: {window_state.count()}" }
            button { onclick: move |_| window_state.increment_count(), "Increment" }
            button { onclick: move |_| window_state.open_window(), "New Window" }
        }
    }
}
