use dioxus::prelude::*;
use dioxus_desktop::tao::keyboard::ModifiersState;
use dioxus_desktop::use_global_shortcut;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let toggled = use_state(cx, || false);
    use_global_shortcut(cx, KeyCode::S, ModifiersState::CONTROL, {
        to_owned![toggled];
        move || toggled.modify(|t| !*t)
    });

    cx.render(rsx!("toggle: {toggled.get()}"))
}
