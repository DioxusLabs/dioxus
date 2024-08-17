fn AvailablePlatforms() -> Element {
    rsx! {
        section { class: "w-full dark:bg-ideblack",
            div { class: "container mx-auto max-w-screen-lg",
                div { class: "relative overflow-x-hidden",
                    div { class: "flex flex-col items-center justify-center text-center max-w-screen-lg mx-auto pb-4",
                        h1 { class: "text-[3.3em] font-bold tracking-tight dark:text-white text-ghdarkmetal pb-4 mb-4 ",
                            "One codebase, every platform."
                        }
                        p { class: "text-xl text-gray-600 dark:text-gray-400 pb-4 max-w-screen-sm",
                            "Dioxus is a React-inspired library for Rust focused on developer experience. Build fast, beautiful, and fully-featured apps for every platform in less time."
                        }
                    }
                    snippets::Snippets {}
                }
            }
            div { class: "max-w-screen-lg mx-auto pb-8 px-2 md:px-16 dark:text-white",
                // div { class: "max-w-screen-xl mx-auto pb-64 px-16 dark:text-white",
                TriShow {
                    left: None,
                    center: None,
                    right: rsx! { "Build for the web using Rust and WebAssembly. As fast as SolidJS and more robust than React. Integrated hot reloading for instant iterations." },
                    to: Route::Docs {
                        child: BookRoute::GettingStartedIndex {},
                    },
                    title: "Web with WASM",
                }
                TriShow {
                    left: None,
                    center: None,
                    right: rsx! { "Lightweight (<2mb) desktop and mobile apps with zero configuration. Choose between WebView or WGPU-enabled renderers. Runs on macOS, Windows, Linux, iOS, and Android." },
                    to: Route::Docs {
                        child: BookRoute::GettingStartedIndex {},
                    },
                    title: "Desktop and Mobile",
                }
                TriShow {
                    to: Route::Docs {
                        child: BookRoute::GettingStartedIndex {},
                    },
                    title: "Terminal User Interfaces",
                    right: rsx! { "Quickly convert any CLI tool to a beautiful interactive user interface with just a few lines of code. Runs anywhere with a terminal." },
                    left: None,
                    center: None,
                }
                TriShow {
                    to: Route::Docs {
                        child: BookRoute::GettingStartedIndex {},
                    },
                    title: "Fullstack Apps",
                    right: rsx! { "Pre-render on the server, and hydrate on the client. Perfect lighthouse scores and performance over 1000x better than Node and Python. Perfect for static site generation or fullstack apps." },
                    left: None,
                    center: None,
                }
                TriShow {
                    to: Route::Docs {
                        child: BookRoute::GettingStartedIndex {},
                    },
                    title: "LiveView",
                    right: rsx! { "Render your app entirely on the server. Zero backend configuration capable of handling thousands of active clients." },
                    left: None,
                    center: None,
                    last: true,
                }
            }
        }
    }
}

#[component]
fn TriShow(
    left: Element,
    center: Element,
    right: Element,
    title: &'static str,
    to: Route,
    last: Option<bool>,
) -> Element {
    rsx! {
        div { class: "w-full flex flex-row justify-center max-w-screen-lg",
            // div { class: "grow basis-0", left }
            TriPadding { last: last.unwrap_or_default(), {center} }
            div { class: "grow basis-0",
                Link { to: to.clone(),
                    div { class: "min-w-lg max-w-screen-md hover:shadow-pop rounded-lg p-8",
                        h2 { class: "text-2xl text-gray-800 font-semibold pb-2 dark:text-gray-100 ",
                            "{title}"
                        }
                        {right}
                    }
                }
            }
        }
    }
}

#[component]
fn TriPadding(children: Element, last: bool) -> Element {
    rsx!(
        div { class: "flex flex-col items-center",
            div { class: "w-0 h-10 border-dashed border border-[#444]" }
            IconSplit {}

            if !last {
                div { class: "w-0 h-full border-dashed border border-[#444]", {children} }
            }
        }
    )
}

#[component]
fn DeveloperExperience() -> Element {
    rsx! (
        section { class: "pt-36 w-full dark:bg-ideblack dark:text-white",
            div { class: "container mx-auto max-w-screen-2xl",
                div { class: "relative",
                    div { class: "flex flex-col max-w-screen-lg mx-auto pb-20",
                        h1 { class: "text-[3.3em] font-bold tracking-tight items-center justify-center text-center dark:text-white text-ghdarkmetal pb-4 mb-4 ",
                            "Redefining developer experience."
                        }
                        div { class: "flex flex-row",
                            p { class: "text-xl text-gray-600 dark:text-gray-400 pb-4 max-w-screen-sm w-1/2",
                                "Dioxus is a React-inspired library for Rust that empowers you to quickly build fast, beautiful, and fully-featured apps for every platform."
                            }
                            p { class: "text-xl text-gray-600 dark:text-gray-400 pb-4 max-w-screen-sm w-1/2",
                                "Dioxus is a React-inspired library for Rust that empowers you to quickly build fast, beautiful, and fully-featured apps for every platform."
                            }
                        }
                    }
                    div { class: "max-w-screen-2xl mx-auto flex flex-row",
                        div { class: "w-1/2" }
                        div { class: "w-1/2",
                            ExperienceText {
                                title: "Integrated Devtools",
                                content: "Hot reloading for instant iteration, automatic code formatting, convert HTML to RSX, and more.",
                            }
                            ExperienceText {
                                title: "Minimal configuration",
                                content: "Start projects with `cargo new`. No build scripts or configuration required for development.",
                            }
                            ExperienceText {
                                title: "",
                                content: "Strong typing with no runtime overhead. Automatically derive props, forms, API clients, and more.",
                            }
                        }
                    }
                }
            }
        }
    )
}

#[component]
fn ExperienceText(title: &'static str, content: &'static str) -> Element {
    rsx!(
        div { class: "pb-12",
            h3 { class: "text-2xl text-gray-800 font-semibold pb-2 dark:text-gray-100 ",
                "{title}"
            }
            p { "{content}" }
        }
    )
}

fn IconSplit() -> Element {
    rsx! {
        svg {
            class: "mx-auto fill-[#444] dark:fill-white",
            version: "1.1",
            view_box: "0 0 24 24",
            width: "24",
            "data-view-component": "true",
            "aria-hidden": "true",
            height: "24",
            path {
                stroke_width: "1.5",
                fill_rule: "evenodd",
                d: "M15.5 11.75a3.5 3.5 0 11-7 0 3.5 3.5 0 017 0zm1.444-.75a5.001 5.001 0 00-9.888 0H2.75a.75.75 0 100 1.5h4.306a5.001 5.001 0 009.888 0h4.306a.75.75 0 100-1.5h-4.306z",
            }
        }
    }
}

fn Stats() -> Element {
    rsx! {
        section { class: "py-12 w-full dark:bg-ideblack",
            div { class: "container mx-auto max-w-screen-lg",
                div { class: "relative ",
                    div { class: "flex flex-col items-center justify-center text-center max-w-screen-lg mx-auto pb-4",
                        // span { class: "text-xl text-blue-300", "Portable" }
                        h1 { class: "text-[3.3em] font-bold tracking-tight dark:text-white text-ghdarkmetal pb-4 mb-4 ",
                            "A vibrant, active community."
                        }
                        p { class: "text-xl text-gray-600 dark:text-gray-400 pb-4 max-w-screen-sm",
                            "Driven by a large, active, and welcoming community."
                        }
                    }
                }
            }
            div { class: "max-w-screen-xl mx-auto py-12 px-2 md:px-16 dark:bg-[#111111] mb-12",
                div { class: "grid grid-cols-2 grid-rows-2 sm:grid-cols-4 sm:grid-rows-1",
                    StatsItem { major: "16k", minor: "Stars" }
                    StatsItem { major: "140k", minor: "Downloads" }
                    StatsItem { major: "206", minor: "Contributors" }
                    StatsItem { major: "1500", minor: "Community Projects" }
                }
            }

            a { href: "https://github.com/dioxuslabs/dioxus/graphs/contributors",
                img {
                    src: "https://contrib.rocks/image?repo=dioxuslabs/dioxus&max=52&columns=13",
                    class: "mx-auto pb-12",
                    alt: "Dioxus Contributors",
                }
            }
        }
    }
}

#[component]
fn StatsItem(major: &'static str, minor: &'static str) -> Element {
    rsx! {
        div { class: "text-center shadow mx-2 rounded-lg py-6 border",
            div { class: "text-5xl font-bold text-gray-800 dark:text-gray-100", {major} }
            div { class: "text-xl text-gray-600 dark:text-gray-400", {minor} }
        }
    }
}
