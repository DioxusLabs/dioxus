use dioxus::prelude::*;

#[component]
pub(crate) fn ChildrenOrLoading(children: Element) -> Element {
    rsx! {
        Stylesheet { href: asset!("/public/loading.css") }
        SuspenseBoundary {
            fallback: |_| rsx! { div { class: "spinner", } },
            {children}
        }
    }
}
