// ANCHOR: all
#![allow(non_snake_case)]

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(MemeEditor);
}

// ANCHOR: meme_editor
fn MemeEditor(cx: Scope) -> Element {
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
            h1 { "Meme Editor" },
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
    let container_style = r#"
        position: relative;
        width: fit-content;
    "#;

    let caption_container_style = r#"
        position: absolute;
        bottom: 0;
        left: 0;
        right: 0;
        padding: 16px 8px;
    "#;

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
    let input_style = r"
        border: none;
        background: cornflowerblue;
        padding: 8px 16px;
        margin: 0;
        border-radius: 4px;
        color: white;
    ";

    cx.render(rsx!(input {
        style: "{input_style}",
        value: "{caption}",
        oninput: move |event| on_input.call(event),
    }))
}
// ANCHOR_END: caption_editor

// ANCHOR_END: all
