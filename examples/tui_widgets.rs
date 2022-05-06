use dioxus::events::FormData;
use dioxus::prelude::*;
use dioxus::tui::widgets::*;
use dioxus::tui::Config;

fn main() {
    dioxus::tui::launch_cfg(app, Config::new());
    // dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let bg_green = use_state(&cx, || false);

    let color = if *bg_green.get() { "green" } else { "red" };
    cx.render(rsx! {
        div{
            width: "100%",
            background_color: "{color}",
            flex_direction: "column",

            Input{
                oninput: |data: FormData| if &data.value == "good"{
                    bg_green.set(true);
                },
                r#type: "checkbox",
                value: "good",
                height: "1px",
                width: "3px",
            }
            Input{
                oninput: |data: FormData| if &data.value == "good"{
                    bg_green.set(true);
                },
                height: "1px",
                width: "30px",
                size: "4",
            }
            Input{
                oninput: |data: FormData| if &data.value == "10"{
                    bg_green.set(true);
                },
                r#type: "range",
                min: "20",
                max: "80",
            }
            Input{
                oninput: |data: FormData| if &data.value == "10"{
                    bg_green.set(true);
                },
                r#type: "number",
                size: "4",
            }
            Input{
                onclick: |_: FormData| bg_green.set(false),
                r#type: "button",
                value: "reset",
                height: "1px",
                width: "30px",
                size: "10",
            }
        }
    })
}
