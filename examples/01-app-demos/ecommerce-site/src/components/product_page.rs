use std::{fmt::Display, str::FromStr};

use crate::api::{fetch_product, Product};
use dioxus::prelude::*;

#[component]
pub fn ProductPage(product_id: ReadSignal<usize>) -> Element {
    let mut quantity = use_signal(|| 1);
    let mut size = use_signal(Size::default);
    let product = use_loader(move || fetch_product(product_id()))?;

    let Product {
        title,
        price,
        description,
        category,
        image,
        rating,
        ..
    } = product();

    rsx! {
        section { class: "py-20",
            div { class: "container mx-auto px-4",
                div { class: "flex flex-wrap -mx-4 mb-24",
                    div { class: "w-full md:w-1/2 px-4 mb-8 md:mb-0",
                        div { class: "relative mb-10",
                            style: "height: 564px;",
                            a { class: "absolute top-1/2 left-0 ml-8 transform translate-1/2",
                                href: "#",
                                icons::icon_0 {}
                            }
                            img { class: "object-cover w-full h-full",
                                alt: "",
                                src: "{image}",
                            }
                            a { class: "absolute top-1/2 right-0 mr-8 transform translate-1/2",
                                href: "#",
                                icons::icon_1 {}
                            }
                        }
                    }
                    div { class: "w-full md:w-1/2 px-4",
                        div { class: "lg:pl-20",
                            div { class: "mb-10 pb-10 border-b",
                                h2 { class: "mt-2 mb-6 max-w-xl text-5xl md:text-6xl font-bold font-heading",
                                    "{title}"
                                }
                                div { class: "mb-8",
                                    "{rating}"
                                }
                                p { class: "inline-block mb-8 text-2xl font-bold font-heading text-blue-300",
                                    span {
                                        "${price}"
                                    }
                                }
                                p { class: "max-w-md text-gray-500",
                                    "{description}"
                                }
                            }
                            div { class: "flex mb-12",
                                div { class: "mr-6",
                                    span { class: "block mb-4 font-bold font-heading text-gray-400 uppercase",
                                        "QTY"
                                    }
                                    div { class: "inline-flex items-center px-4 font-semibold font-heading text-gray-500 border border-gray-200 focus:ring-blue-300 focus:border-blue-300 rounded-md",
                                        button { class: "py-2 hover:text-gray-700",
                                            onclick: move |_| quantity += 1,
                                            icons::icon_2 {}
                                        }
                                        input { class: "w-12 m-0 px-2 py-4 text-center md:text-right border-0 focus:ring-transparent focus:outline-hidden rounded-md",
                                            placeholder: "1",
                                            r#type: "number",
                                            value: "{quantity}",
                                            oninput: move |evt| if let Ok(as_number) = evt.value().parse() { quantity.set(as_number) },
                                        }
                                        button { class: "py-2 hover:text-gray-700",
                                            onclick: move |_| quantity -= 1,
                                            icons::icon_3 {}
                                        }
                                    }
                                }
                                div {
                                    span { class: "block mb-4 font-bold font-heading text-gray-400 uppercase",
                                        "Size"
                                    }
                                    select { class: "pl-6 pr-10 py-4 font-semibold font-heading text-gray-500 border border-gray-200 focus:ring-blue-300 focus:border-blue-300 rounded-md",
                                        id: "",
                                        name: "",
                                        onchange: move |evt| {
                                            if let Ok(new_size) = evt.value().parse() {
                                                size.set(new_size);
                                            }
                                        },
                                        option {
                                            value: "1",
                                            "Medium"
                                        }
                                        option {
                                            value: "2",
                                            "Small"
                                        }
                                        option {
                                            value: "3",
                                            "Large"
                                        }
                                    }
                                }
                            }
                            div { class: "flex flex-wrap -mx-4 mb-14 items-center",
                                div { class: "w-full xl:w-2/3 px-4 mb-4 xl:mb-0",
                                    a { class: "block bg-orange-300 hover:bg-orange-400 text-center text-white font-bold font-heading py-5 px-8 rounded-md uppercase transition duration-200",
                                        href: "#",
                                        "Add to cart"
                                    }
                                }
                            }
                            div { class: "flex items-center",
                                span { class: "mr-8 text-gray-500 font-bold font-heading uppercase",
                                    "SHARE IT"
                                }
                                a { class: "mr-1 w-8 h-8",
                                    href: "#",
                                    img {
                                        alt: "",
                                        src: "https://shuffle.dev/yofte-assets/buttons/facebook-circle.svg",
                                    }
                                }
                                a { class: "mr-1 w-8 h-8",
                                    href: "#",
                                    img {
                                        alt: "",
                                        src: "https://shuffle.dev/yofte-assets/buttons/instagram-circle.svg",
                                    }
                                }
                                a { class: "w-8 h-8",
                                    href: "#",
                                    img {
                                        src: "https://shuffle.dev/yofte-assets/buttons/twitter-circle.svg",
                                        alt: "",
                                    }
                                }
                            }
                        }
                    }
                }
                div {
                    ul { class: "flex flex-wrap mb-16 border-b-2",
                        li { class: "w-1/2 md:w-auto",
                            a { class: "inline-block py-6 px-10 bg-white text-gray-500 font-bold font-heading shadow-2xl",
                                href: "#",
                                "Description"
                            }
                        }
                        li { class: "w-1/2 md:w-auto",
                            a { class: "inline-block py-6 px-10 text-gray-500 font-bold font-heading",
                                href: "#",
                                "Customer reviews"
                            }
                        }
                        li { class: "w-1/2 md:w-auto",
                            a { class: "inline-block py-6 px-10 text-gray-500 font-bold font-heading",
                                href: "#",
                                "Shipping &amp; returns"
                            }
                        }
                        li { class: "w-1/2 md:w-auto",
                            a { class: "inline-block py-6 px-10 text-gray-500 font-bold font-heading",
                                href: "#",
                                "Brand"
                            }
                        }
                    }
                    h3 { class: "mb-8 text-3xl font-bold font-heading text-blue-300",
                        "{category}"
                    }
                    p { class: "max-w-2xl text-gray-500",
                        "{description}"
                    }
                }
            }
        }
    }
}

#[derive(Default)]
enum Size {
    Small,
    #[default]
    Medium,
    Large,
}

impl Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Size::Small => "small".fmt(f),
            Size::Medium => "medium".fmt(f),
            Size::Large => "large".fmt(f),
        }
    }
}

impl FromStr for Size {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Size::*;
        match s.to_lowercase().as_str() {
            "small" => Ok(Small),
            "medium" => Ok(Medium),
            "large" => Ok(Large),
            _ => Err(()),
        }
    }
}

mod icons {
    use super::*;

    pub(super) fn icon_0() -> Element {
        rsx! {
            svg { class: "w-6 h-6",
                view_box: "0 0 24 23",
                xmlns: "http://www.w3.org/2000/svg",
                height: "23",
                fill: "none",
                width: "24",
                path {
                    stroke: "black",
                    fill: "black",
                    d: "M2.01328 18.9877C2.05682 16.7902 2.71436 12.9275 6.3326 9.87096L6.33277 9.87116L6.33979 9.86454L6.3398 9.86452C6.34682 9.85809 8.64847 7.74859 13.4997 7.74859C13.6702 7.74859 13.8443 7.75111 14.0206 7.757L14.0213 7.75702L14.453 7.76978L14.6331 7.77511V7.59486V3.49068L21.5728 10.5736L14.6331 17.6562V13.6558V13.5186L14.4998 13.4859L14.1812 13.4077C14.1807 13.4075 14.1801 13.4074 14.1792 13.4072M2.01328 18.9877L14.1792 13.4072M2.01328 18.9877C7.16281 11.8391 14.012 13.3662 14.1792 13.4072M2.01328 18.9877L14.1792 13.4072M23.125 10.6961L23.245 10.5736L23.125 10.4512L13.7449 0.877527L13.4449 0.571334V1V6.5473C8.22585 6.54663 5.70981 8.81683 5.54923 8.96832C-0.317573 13.927 0.931279 20.8573 0.946581 20.938L0.946636 20.9383L1.15618 22.0329L1.24364 22.4898L1.47901 22.0885L2.041 21.1305L2.04103 21.1305C4.18034 17.4815 6.71668 15.7763 8.8873 15.0074C10.9246 14.2858 12.6517 14.385 13.4449 14.4935V20.1473V20.576L13.7449 20.2698L23.125 10.6961Z",
                    stroke_width: "0.35",
                }
            }
        }
    }

    pub(super) fn icon_1() -> Element {
        rsx! {
            svg { class: "w-6 h-6",
                height: "27",
                view_box: "0 0 27 27",
                fill: "none",
                width: "27",
                xmlns: "http://www.w3.org/2000/svg",
                path {
                    d: "M13.4993 26.2061L4.70067 16.9253C3.9281 16.1443 3.41815 15.1374 3.24307 14.0471C3.06798 12.9568 3.23664 11.8385 3.72514 10.8505V10.8505C4.09415 10.1046 4.63318 9.45803 5.29779 8.96406C5.96241 8.47008 6.73359 8.14284 7.54782 8.00931C8.36204 7.87578 9.19599 7.93978 9.98095 8.19603C10.7659 8.45228 11.4794 8.89345 12.0627 9.48319L13.4993 10.9358L14.9359 9.48319C15.5192 8.89345 16.2327 8.45228 17.0177 8.19603C17.8026 7.93978 18.6366 7.87578 19.4508 8.00931C20.265 8.14284 21.0362 8.47008 21.7008 8.96406C22.3654 9.45803 22.9045 10.1046 23.2735 10.8505V10.8505C23.762 11.8385 23.9306 12.9568 23.7556 14.0471C23.5805 15.1374 23.0705 16.1443 22.298 16.9253L13.4993 26.2061Z",
                    stroke: "black",
                    stroke_width: "1.5",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                }
            }
        }
    }

    pub(super) fn icon_2() -> Element {
        rsx! {
            svg {
                view_box: "0 0 12 12",
                height: "12",
                width: "12",
                fill: "none",
                xmlns: "http://www.w3.org/2000/svg",
                g {
                    opacity: "0.35",
                    rect {
                        height: "12",
                        x: "5",
                        fill: "currentColor",
                        width: "2",
                    }
                    rect {
                        fill: "currentColor",
                        width: "2",
                        height: "12",
                        x: "12",
                        y: "5",
                        transform: "rotate(90 12 5)",
                    }
                }
            }
        }
    }

    pub(super) fn icon_3() -> Element {
        rsx! {
            svg {
                width: "12",
                fill: "none",
                view_box: "0 0 12 2",
                height: "2",
                xmlns: "http://www.w3.org/2000/svg",
                g {
                    opacity: "0.35",
                    rect {
                        transform: "rotate(90 12 0)",
                        height: "12",
                        fill: "currentColor",
                        x: "12",
                        width: "2",
                    }
                }
            }
        }
    }
}
