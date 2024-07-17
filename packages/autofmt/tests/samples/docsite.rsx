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

                div { class: "hidden md:flex h-full justify-end ml-2 flex-1",
                    div { class: "hidden md:flex items-center",
                        Search {}
                        div { class: "hidden lg:flex items-center border-l border-gray-200 ml-4 pl-4 dark:border-gray-800",
                            label {
                                class: "sr-only",
                                id: "headlessui-listbox-label-2",
                                "Theme"
                            }
                            Link {
                                to: "https://discord.gg/XgGxMSkvUM",
                                class: "block text-gray-400 hover:text-gray-500 dark:hover:text-gray-300",
                                new_tab: true,
                                span { class: "sr-only", "Dioxus on Discord" }
                                crate::icons::DiscordLogo {}
                            }
                            Link {
                                to: "https://github.com/dioxuslabs/dioxus",
                                class: "ml-4 block text-gray-400 hover:text-gray-500 dark:hover:text-gray-300",
                                new_tab: true,
                                span { class: "sr-only", "Dioxus on GitHub" }
                                crate::icons::Github2 {}
                            }
                        }
                        div { class: "hidden lg:flex items-center border-l border-gray-200 ml-4 pl-6 dark:border-gray-800",
                            label {
                                class: "sr-only",
                                id: "headlessui-listbox-label-2",
                                "Theme"
                            }
                            Link {
                                to: Route::Deploy {},
                                class: "md:ml-0 md:py-2 md:px-3 bg-blue-500 ml-4 text-lg md:text-sm text-white rounded font-semibold",
                                "DEPLOY"
                            }
                            if LOGGED_IN() {
                                Link { to: Route::Homepage {},
                                    img {
                                        src: "https://avatars.githubusercontent.com/u/10237910?s=40&v=4",
                                        class: "ml-4 h-10 rounded-full w-auto"
                                    }
                                }
                            }
                        }
                    }
                }
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

    let _ = rsx! {
        SidebarChapter { chapter }
    };

    rsx! {
        SidebarChapter { chapter }
    };

    rsx! {
        div {}
    };

    rsx! { "hi" }

    rsx! {
        div { class: "full-chapter pb-4 mb-6",
            if let Some(url) = &link.location {
                Link {
                    onclick: move |_| *SHOW_SIDEBAR.write() = false,
                    to: Route::Docs { child: *url },
                    h3 { class: "font-semibold mb-4", "{link.name}" }
                }
            }
            ul { class: "ml-1", {sections} }
        }
    }
}
