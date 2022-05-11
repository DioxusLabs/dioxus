use dioxus::desktop::use_window;
use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let window = use_window(&cx);

    let level = use_state(&cx, || 1.0);

    window.set_zoom_level(*level.get());
    cx.render(rsx! {
        input {
            r#type: "number",
            value: "{level_display}",
            oninput: |e| {
                level.set(e.value.parse::<f64>().unwrap_or_default())
            }
        }
    })
}
