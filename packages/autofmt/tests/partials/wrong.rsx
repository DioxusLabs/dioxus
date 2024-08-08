#[component]
fn SidebarSection() -> Element {
    rsx! {
        div {
            onclick: move |_| {
                .doesnt_work()
            }
        }
    }
}
