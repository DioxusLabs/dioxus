use dioxus::prelude::*;
use dioxus_desktop::{DesktopContext, WindowConfig};

#[path = "./utils.rs"]
mod utils;

fn main() {
    #[cfg(not(windows))]
    utils::check_app_exits(app);
}

static MOUNTED_WINDOWS: GlobalSignal<Vec<usize>> = Signal::global(Vec::new);
static CLOSED_WINDOWS: GlobalSignal<Vec<usize>> = Signal::global(Vec::new);

fn app() -> Element {
    let desktop_context: DesktopContext = consume_context();
    let mut windows = use_signal(Vec::<usize>::new);
    let mut opened_third = use_signal(|| false);

    use_hook({
        let desktop_context = desktop_context.clone();
        move || {
            spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                windows.write().push(0);

                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                windows.write().push(1);

                tokio::time::sleep(std::time::Duration::from_millis(3500)).await;
                MOUNTED_WINDOWS.with(|mounted| {
                    assert!(
                        mounted.contains(&2),
                        "child 2 should mount after child 0 closes"
                    );
                });
                desktop_context.close();
            });
        }
    });

    use_effect(move || {
        let closed_windows = CLOSED_WINDOWS();
        if closed_windows.contains(&0) && !opened_third() {
            opened_third.set(true);
            windows.write().push(2);
        }
    });

    rsx! {
        for id in windows() {
            Window {
                key: "{id}",
                config: hidden_window_config(),
                onclose: move |_| {
                    windows.write().retain(|window_id| *window_id != id);
                    CLOSED_WINDOWS.write().push(id);
                },
                ChildWindow { id }
            }
        }
    }
}

#[component]
fn ChildWindow(id: usize) -> Element {
    let desktop_context: DesktopContext = consume_context();

    use_hook(move || {
        MOUNTED_WINDOWS.write().push(id);

        if id == 0 {
            spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                desktop_context.close();
            });
        }
    });

    rsx! {
        div { "child {id}" }
    }
}

fn hidden_window_config() -> WindowConfig {
    WindowConfig::new()
        .with_window(dioxus_desktop::tao::window::WindowBuilder::new().with_visible(false))
}
