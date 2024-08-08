/// dont format this component
#[rustfmt::skip]
#[component]
fn SidebarSection() -> Element {
    rsx! {
        div {
            "hi" div {} div {}
        }
    }
}

/// dont format this component
#[component]
fn SidebarSection() -> Element {
    // format this
    rsx! {
        div { "hi" }
    }

    // and this
    rsx! {
        div {
            "hi"
            div {}
            div {}
        }
    }

    // but not this
    #[rustfmt::skip]
    rsx! {
        div {
            "hi" div {} div {}
        }
    }
}
