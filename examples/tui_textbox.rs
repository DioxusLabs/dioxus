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
            Input{
                r#type: "checkbox",
            }
            Input{
                oninput: |data: FormData| if &data.value == "good"{
                    bg_green.set(true);
                }
                else{
                    bg_green.set(false);
                }
            }
        }
    })
}
