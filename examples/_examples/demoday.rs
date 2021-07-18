use dioxus_core as dioxus;
use dioxus_html as dioxus_elements;
use dioxus_html::*;
use dioxus_web::{dioxus::prelude::*, WebsysRenderer};

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(WebsysRenderer::start(App))
}

fn App(cx: Context<()>) -> DomTree {
    cx.render(rsx! {
        div { class: "dark:bg-gray-800 bg-white relative h-screen"
            NavBar {}
            {(0..10).map(|f| rsx!(Landing { key: "{f}" }))}
        }
    })
}

fn NavBar(cx: Context<()>) -> DomTree {
    cx.render(rsx!{
        header { class: "h-24 sm:h-32 flex items-center z-30 w-full"
            div { class: "container mx-auto px-6 flex items-center justify-between"
                div { class: "uppercase text-gray-800 dark:text-white font-black text-3xl"
                    svg { focusable:"false" width:"100" height:"100" viewBox: "0 0 512 309"
                        path { fill: "#000"
                            d: "M120.81 80.561h96.568v7.676h-87.716v57.767h82.486v7.675h-82.486v63.423h88.722v7.675H120.81V80.561zm105.22 0h10.26l45.467 63.423L328.23 80.56L391.441 0l-103.85 150.65l53.515 74.127h-10.663l-48.686-67.462l-48.888 67.462h-10.461l53.917-74.128l-50.296-70.088zm118.898 7.676V80.56h110.048v7.676h-50.699v136.54h-8.852V88.237h-50.497zM0 80.56h11.065l152.58 228.323l-63.053-84.107L9.254 91.468l-.402 133.31H0V80.56zm454.084 134.224c-1.809 0-3.165-1.4-3.165-3.212c0-1.81 1.356-3.212 3.165-3.212c1.83 0 3.165 1.401 3.165 3.212c0 1.811-1.335 3.212-3.165 3.212zm8.698-8.45h4.737c.064 2.565 1.937 4.29 4.693 4.29c3.079 0 4.823-1.854 4.823-5.325v-21.99h4.823v22.011c0 6.252-3.617 9.853-9.603 9.853c-5.62 0-9.473-3.493-9.473-8.84zm25.384-.28h4.78c.409 2.953 3.294 4.828 7.45 4.828c3.875 0 6.717-2.005 6.717-4.764c0-2.371-1.809-3.794-5.921-4.764l-4.005-.97c-5.62-1.316-8.181-4.032-8.181-8.602c0-5.54 4.521-9.227 11.303-9.227c6.308 0 10.916 3.686 11.196 8.925h-4.694c-.452-2.867-2.95-4.657-6.567-4.657c-3.81 0-6.35 1.833-6.35 4.635c0 2.22 1.635 3.493 5.683 4.441l3.423.841c6.373 1.488 9 4.075 9 8.753c0 5.95-4.607 9.68-11.97 9.68c-6.89 0-11.52-3.558-11.864-9.12z" 
                        }
                    }
                }
                div { class:"flex items-center"
                    nav { class: "font-sen text-gray-800 dark:text-white uppercase text-lg lg:flex items-center hidden"
                        a { href: "#", class:"py-2 px-6 flex text-indigo-500 border-b-2 border-indigo-500"
                            "Home"
                        }
                        a { href: "#", class: "py-2 px-6 flex hover:text-indigo-500"
                            "Watch"
                        }
                        a { href: "#", class: "py-2 px-6 flex hover:text-indigo-500"
                            "Product"
                        }
                        a { href: "#", class: "py-2 px-6 flex hover:text-indigo-500"
                            "Contact"
                        }
                        a { href: "#", class: "py-2 px-6 flex hover:text-indigo-500"
                            "Career"
                        }
                    }
                    button { class: "lg:hidden flex flex-col ml-4"
                        span { class: "w-6 h-1 bg-gray-800 dark:bg-white mb-1" }
                        span { class: "w-6 h-1 bg-gray-800 dark:bg-white mb-1" }
                        span { class: "w-6 h-1 bg-gray-800 dark:bg-white mb-1" }
                    }
                }
            }
        }
    })
}

fn Landing(cx: Context<()>) -> DomTree {
    cx.render(rsx!{
        div { class: "bg-white dark:bg-gray-800 flex relative z-20 items-center"
            div { class: "container mx-auto px-6 flex flex-col justify-between items-center relative py-8"
                div { class: "flex flex-col"
                    h1 { class: "font-light w-full uppercase text-center text-4xl sm:text-5xl dark:text-white text-gray-800"
                        "The Dioxus Framework for Production"
                    }
                    h2{ class: "font-light max-w-2xl mx-auto w-full text-xl dark:text-white text-gray-500 text-center py-8"
                        "Next.js gives you the best developer experience with all the features you need for production: \n
                        hybrid static &amp; server rendering, TypeScript support, smart bundling, route pre-fetching, and \n
                        more. No config needed."
                    }
                    div { class: "flex items-center justify-center mt-4"
                        a { href: "#" class: "uppercase py-2 px-4 bg-gray-800 border-2 border-transparent text-white text-md mr-4 hover:bg-gray-900"
                            "Get started"
                        }
                        a{ href: "#" class: "uppercase py-2 px-4 bg-transparent border-2 border-gray-800 text-gray-800 dark:text-white hover:bg-gray-800 hover:text-white text-md"
                            "Documentation"
                        }
                    }
                }
                div { class: "block w-full mx-auto mt-6 md:mt-0 relative"
                    img { src: "/images/object/12.svg" class: "max-w-xs md:max-w-2xl m-auto" }
                }
            }
        }
    })
}
