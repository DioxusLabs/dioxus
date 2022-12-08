use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! { generic_child::<i32>{} })
}

fn generic_child<T>(cx: Scope) -> Element {
    cx.render(rsx! { div {} })
}
