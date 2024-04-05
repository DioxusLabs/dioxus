#[component]
fn SidebarSection() -> Element {
    rsx! {
        if let Some(url) = &link.location {
            "hi {url}"
        }

        if val.is_empty() {
            "No content"
        }
    }
}
