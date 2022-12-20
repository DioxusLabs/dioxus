// ANCHOR: all
#![allow(non_snake_case)]

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

// ANCHOR: DarkMode_struct
struct DarkMode(bool);
// ANCHOR_END: DarkMode_struct

#[rustfmt::skip]
pub fn App(cx: Scope) -> Element {
    // ANCHOR: context_provider
use_shared_state_provider(cx, || DarkMode(false));
    // ANCHOR_END: context_provider

    let is_dark_mode = use_is_dark_mode(cx);

    let wrapper_style = if is_dark_mode {
        r"
            background: #222;
            min-height: 100vh;
        "
    } else {
        r""
    };

    cx.render(rsx!(div {
        style: "{wrapper_style}",
        DarkModeToggle {},
        MemeEditor {},
    }))
}

#[rustfmt::skip]
pub fn use_is_dark_mode(cx: &ScopeState) -> bool {
    // ANCHOR: use_context
let dark_mode_context = use_shared_state::<DarkMode>(cx);
    // ANCHOR_END: use_context

    dark_mode_context
        .map(|context| context.read().0)
        .unwrap_or(false)
}

// ANCHOR: toggle
pub fn DarkModeToggle(cx: Scope) -> Element {
    let dark_mode = use_shared_state::<DarkMode>(cx).unwrap();

    let style = if dark_mode.read().0 {
        "color:white"
    } else {
        ""
    };

    cx.render(rsx!(label {
        style: "{style}",
        "Dark Mode",
        input {
            r#type: "checkbox",
            oninput: move |event| {
                let is_enabled = event.value == "true";
                dark_mode.write().0 = is_enabled;
            },
        },
    }))
}
// ANCHOR_END: toggle

// ANCHOR: meme_editor
fn MemeEditor(cx: Scope) -> Element {
    let is_dark_mode = use_is_dark_mode(cx);
    let heading_style = if is_dark_mode { "color: white" } else { "" };

    let container_style = r"
        display: flex;
        flex-direction: column;
        gap: 16px;
        margin: 0 auto;
        width: fit-content;
    ";

    let caption = use_state(cx, || "me waiting for my rust code to compile".to_string());

    cx.render(rsx! {
        div {
            style: "{container_style}",
            h1 {
                style: "{heading_style}",
                "Meme Editor"
            },
            Meme {
                caption: caption,
            },
            CaptionEditor {
                caption: caption,
                on_input: move |event: FormEvent| {caption.set(event.value.clone());},
            },
        }
    })
}
// ANCHOR_END: meme_editor

// ANCHOR: meme_component
#[inline_props]
fn Meme<'a>(cx: Scope<'a>, caption: &'a str) -> Element<'a> {
    let container_style = r"
        position: relative;
        width: fit-content;
    ";

    let caption_container_style = r"
        position: absolute;
        bottom: 0;
        left: 0;
        right: 0;
        padding: 16px 8px;
    ";

    let caption_style = r"
        font-size: 32px;
        margin: 0;
        color: white;
        text-align: center;
    ";

    cx.render(rsx!(
        div {
            style: "{container_style}",
            img {
                src: "https://i.imgflip.com/2zh47r.jpg",
                height: "500px",
            },
            div {
                style: "{caption_container_style}",
                p {
                    style: "{caption_style}",
                    "{caption}"
                }
            }
        }
    ))
}
// ANCHOR_END: meme_component

// ANCHOR: caption_editor
#[inline_props]
fn CaptionEditor<'a>(
    cx: Scope<'a>,
    caption: &'a str,
    on_input: EventHandler<'a, FormEvent>,
) -> Element<'a> {
    let is_dark_mode = use_is_dark_mode(cx);

    let colors = if is_dark_mode {
        r"
            background: cornflowerblue;
            color: white;
        "
    } else {
        r"
            background: #def;
            color: black;
        "
    };

    let input_style = r"
        border: none;
        padding: 8px 16px;
        margin: 0;
        border-radius: 4px;
    ";

    cx.render(rsx!(input {
        style: "{input_style}{colors}",
        value: "{caption}",
        oninput: move |event| on_input.call(event),
    }))
}
// ANCHOR_END: caption_editor

// ANCHOR_END: all
