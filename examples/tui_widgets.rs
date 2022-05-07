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
            align_items: "center",

            Input{
                oninput: |data: FormData| if &data.value == "good"{
                    bg_green.set(true);
                } else{
                    bg_green.set(false);
                },
                r#type: "checkbox",
                value: "good",
                height: "3px",
                width: "3px",
                checked: "true",
            }
            Input{
                oninput: |data: FormData| if &data.value == "hello world"{
                    bg_green.set(true);
                } else{
                    bg_green.set(false);
                },
                height: "3px",
                width: "13px",
                maxlength: "11",
            }
            Input{
                oninput: |data: FormData| {
                    if (data.value.parse::<f32>().unwrap() - 40.0).abs() < 5.0 {
                        bg_green.set(true);
                    } else{
                        bg_green.set(false);
                    }
                },
                r#type: "range",
                min: "20",
                max: "80",
            }
            Input{
                oninput: |data: FormData| {
                    if data.value == "10"{
                        bg_green.set(true);
                    } else{
                        bg_green.set(false);
                    }
                },
                r#type: "number",
                maxlength: "4",
            }
            Input{
                oninput: |data: FormData| {
                    if data.value == "hello world"{
                        bg_green.set(true);
                    } else{
                        bg_green.set(false);
                    }
                },
                r#type: "password",
                width: "13px",
                height: "3px",
                maxlength: "11",
            }
            Input{
                onclick: |_: FormData| bg_green.set(false),
                r#type: "button",
                value: "green",
                height: "3px",
                width: "7px",
            }
        }
    })
}
