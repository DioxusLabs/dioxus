use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    let level = use_state(|| 1.0);

    cx.render(rsx! {
        input {
            r#type: "number",
            value: "{level}",
            oninput: |e| {
                if let Ok(new_zoom) = e.value().parse::<f64>() {
                    level.set(new_zoom);
                    dioxus_desktop::window().webview.zoom(new_zoom);
                }
            }
        }
    })
}
