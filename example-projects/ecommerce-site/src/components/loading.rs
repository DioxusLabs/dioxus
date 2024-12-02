use dioxus::prelude::*;

#[component]
pub(crate) fn ChildrenOrLoading(children: Element) -> Element {
    rsx! {
        document::Link {
            rel: "stylesheet",
            href: asset!("/public/loading.css")
        }
        SuspenseBoundary {
            fallback: |context: SuspenseContext| {
                rsx! {
                    if let Some(placeholder) = context.suspense_placeholder() {
                        {placeholder}
                    } else {
                        LoadingIndicator {}
                    }
                }
            },
            {children}
        }
    }
}

#[component]
fn LoadingIndicator() -> Element {
    rsx! {
        div {
            class: "spinner",
        }
    }
}
