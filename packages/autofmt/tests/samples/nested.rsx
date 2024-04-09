//! some basic test cases with nested rsx!

fn App() -> Element {
    let mut count = use_signal(|| 0);
    let mut text = use_signal(|| "...".to_string());

    rsx! {
        div {
            div { "hi" }
            div {
                header: rsx! {
                    div { class: "max-w-lg lg:max-w-2xl mx-auto mb-16 text-center",
                        "gomg"
                        "hi!!"
                        "womh"
                    }
                },
                header: rsx! {
                    div { class: "max-w-lg lg:max-w-2xl mx-auto mb-16 text-center",
                        "gomg"
                        "hi!!"
                        "womh"
                    }
                },
                header: rsx! {
                    div { class: "max-w-lg lg:max-w-2xl mx-auto mb-16 text-center",
                        "gomg"
                        // "hi!!"
                        "womh"
                    }
                },
                onclick: move |_| {
                    rsx! {
                        div { class: "max-w-lg lg:max-w-2xl mx-auto mb-16 text-center",
                            "gomg"
                            "hi!!"
                            "womh"
                        }
                    };
                    println!("hi")
                },
                "hi"
            }
            ContentList { header, content: &BLOG_POSTS, readmore: true }
        }
        Component {
            header: rsx! {
                h1 { "hi" }
                h1 { "hi" }
            },
            blah: rsx! {
                h1 { "hi" }
            },
            blah: rsx! {
                h1 { "hi" }
            },
            blah: rsx! {
                h1 { "hi" }
            },
            blah: rsx! { "hi" },
            blah: rsx! {
                h1 { "hi" }
                Component {
                    header: rsx! {
                        Component {
                            header: rsx! {
                                div { "hi" }
                                h3 { "hi" }
                                p { "hi" }
                                Component {
                                    onrender: move |_| {
                                        count += 1;
                                        let abc = rsx! {
                                            div {
                                                h1 { "hi" }
                                                "something nested?"
                                                Component {
                                                    onrender: move |_| {
                                                        count2 += 1;
                                                        rsx! {
                                                            div2 {
                                                                h12 { "hi" }
                                                                "so22mething nested?"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        };
                                        rsx! {
                                            div {
                                                h1 { "hi" }
                                                "something nested?"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            onrender: move |_| {
                count += 1;
                rsx! {
                    div {
                        h1 { "hi" }
                        "something nested?"
                    }
                    Component2 {
                        header2: rsx! {
                            h1 { "hi1" }
                            h1 { "hi2" }
                        },
                        onrender2: move |_| {
                            count2 += 1;
                            rsx! {
                                div2 {
                                    h12 { "hi" }
                                    "so22mething nested?"
                                }
                            }
                        },
                        {rsx! {
                            div2 {
                                h12 { "hi" }
                                "so22mething nested?"
                            }
                        }}
                    }
                }
            },
            div {
                onclick: move |_| {
                    let val = rsx! {
                        div {
                            h1 { "hi" }
                            "something nested?"
                        }
                    };
                }
            }
        }
    }
}
