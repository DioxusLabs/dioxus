use dioxus::prelude::*;

#[allow(non_snake_case)]
pub fn FallbackNamedNavigation(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "A named navigation error has occurred!" }
        p {
            "If you see this message, the application you are using has a bug. Please report it to "
            "the developer so they can fix it."
            strong { "Thank you!" }
        }
    })
}
