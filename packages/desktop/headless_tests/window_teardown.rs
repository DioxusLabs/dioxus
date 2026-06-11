//! Regression test for window teardown ordering: a window's main-thread state must stay alive
//! for as long as any `DesktopContext` for it exists, so proxied window/webview calls always
//! return real values — even through a handle held after the window was closed — and the app
//! still exits once every handle is gone.

use dioxus::prelude::*;
use dioxus_desktop::{Config, WindowBuilder, window};

#[path = "./utils.rs"]
mod utils;

fn main() {
    utils::check_app_exits(app);
}

fn child() -> Element {
    rsx! {
        div { "child" }
    }
}

fn hidden_child_config() -> Config {
    Config::new().with_window(WindowBuilder::new().with_visible(false))
}

fn app() -> Element {
    use_hook(|| {
        spawn(async move {
            // Stress the spawn/abort ordering: close each window the moment its context
            // resolves. A lost abort would leak the window's VirtualDom task (and with it the
            // window's handles), hanging the exit at the end of this test.
            for _ in 0..5 {
                let child_window = window()
                    .new_window(VirtualDom::new(child), hidden_child_config)
                    .await;
                child_window.close();
            }

            // Hold a context for a window that has been told to close. The window's
            // main-thread state must stay alive until this handle drops, so proxied calls
            // keep returning real values instead of fabricated fallbacks.
            let child_window = window()
                .new_window(VirtualDom::new(child), hidden_child_config)
                .await;
            child_window.close();
            // Give the close and the VirtualDom task abort time to land.
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            // The closed-window fallback used to fabricate `PhysicalSize::default()` (0 x 0);
            // the real — hidden but alive — window still reports its actual size.
            let size = child_window.inner_size();
            assert_ne!(
                size.width, 0,
                "a closed-but-held window must report its real size"
            );
            // These used to fabricate `Ok((0, 0))` / `""` once the window left the map.
            child_window
                .inner_position()
                .expect("inner_position on a held window handle");
            let _ = child_window.title();
            drop(child_window);

            // With every child handle dropped, closing the root window must drain the app.
            window().close();
        });
    });

    rsx! {
        div { "parent" }
    }
}
