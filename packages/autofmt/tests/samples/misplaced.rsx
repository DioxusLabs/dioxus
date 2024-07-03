pub(crate) fn Nav() -> Element {
    rsx! {
        SearchModal {}
        header {
            class: "sticky top-0 z-30 bg-white dark:text-gray-200 dark:bg-ideblack border-b dark:border-stone-700 h-16 bg-opacity-80 backdrop-blur-sm",
            class: if HIGHLIGHT_NAV_LAYOUT() { "border border-orange-600 rounded-md" },
            div { class: "lg:py-2 px-2 max-w-screen-2xl mx-auto flex items-center justify-between text-sm leading-6 h-16",
                button {
                    class: "bg-gray-100 rounded-lg p-2 mr-4 lg:hidden my-3 h-10 flex items-center text-lg z-[100]",
                    class: if !SHOW_DOCS_NAV() { "hidden" },
                    onclick: move |_| {
                        let mut sidebar = SHOW_SIDEBAR.write();
                        *sidebar = !*sidebar;
                    },
                    MaterialIcon {
                        name: "menu",
                        size: 24,
                        color: MaterialIconColor::Dark
                    }
                }
                div { class: "flex z-50 md:flex-1 px-2", LinkList {} }
            }
        }
    }
}

#[component]
fn SidebarSection(chapter: &'static SummaryItem<BookRoute>) -> Element {
    let link = chapter.maybe_link()?;
    let sections = link.nested_items.iter().map(|chapter| {
        rsx! {
            SidebarChapter { chapter }
        }
    });
}
