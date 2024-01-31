use dioxus::prelude::*;

#[component]
pub fn Explainer<'a>(
    cx: Scope<'a>,
    invert: bool,
    title: &'static str,
    content: Element,
    flasher: Element,
) -> Element {
    // pt-5 sm:pt-24 lg:pt-24

    let mut right = rsx! {
        div { class: "relative w-1/2", {flasher} }
    };

    let align = match invert {
        true => "mr-auto ml-16",
        false => "ml-auto mr-16",
    };

    let mut left = rsx! {
        div { class: "relative w-1/2 {align} max-w-md leading-8",
            h2 { class: "mb-6 text-3xl leading-tight md:text-4xl md:leading-tight lg:text-3xl lg:leading-tight font-heading font-mono font-bold",
                "{title}"
            }
            {content}
        }
    };

    if *invert {
        std::mem::swap(&mut left, &mut right);
    }

    rsx! {
        div { class: "flex flex-wrap items-center dark:text-white py-16 border-t font-light",
            {left},
            {right}
        }
    }
}
