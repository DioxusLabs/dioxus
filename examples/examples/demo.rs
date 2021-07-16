//! An example where the dioxus vdom is running in a native thread, interacting with webview
//! Content is passed from the native thread into the webview
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
fn main() {
    dioxus_desktop::launch(
        |builder| {
            builder
                .title("Test Dioxus App")
                .size(320, 480)
                .resizable(false)
                .debug(true)
        },
        (),
        Example,
    )
    .expect("Webview finished");
}

// static Example: FC<()> = |cx| {
//     cx.render(html! {
//         <div>
//         <svg class="octicon octicon-star v-align-text-bottom"
//         viewBox="0 0 14 16" version="1.1"
//         width="14" height="16"
//         xmlns="http://www.w3.org/2000/svg"
//         >

//         <path
//         d="M14 6l-4.9-.64L7 1 4.9 5.36 0 6l3.6 3.26L2.67 14"
//         xmlns="http://www.w3.org/2000/svg"
//         >
//         </path>

//         </svg>
//         </div>
//     })
// };
static Example: FC<()> = |cx| {
    cx.render(rsx! {
        div  {
            class: "flex items-center justify-center flex-col"
            div {
                class: "flex items-center justify-center"
                div {
                    class: "flex flex-col bg-white rounded p-4 w-full max-w-xs"
                    div { class: "font-bold text-xl", "Example desktop app" }
                    div { class: "text-sm text-gray-500", "This is running natively" }
                    div {
                        class: "flex flex-row items-center justify-center mt-6"
                        div { class: "font-medium text-6xl", "100%" }
                    }
                    div {
                        class: "flex flex-row justify-between mt-6"
                        a {
                            href: "https://www.dioxuslabs.com"
                            class: "underline"
                            "Made with dioxus"
                        }
                    }
                    ul {
                        {(0..10).map(|f| rsx!(li {
                            key: "{f}"
                            "{f}"
                        }))}
                    }
                }
            }
        }
    })
};
