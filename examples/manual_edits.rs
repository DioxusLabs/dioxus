/*
Example: Manual Edits

It's possible to manually provide a stream of DomEdits to a Dioxus Renderer. All renderers are designed to accept a stream
of DomEdits that abstract over a stack machine. This allows the VirtualDOM to exist entirely separately from the RealDOM,
though features like NodeRefs and NativeEvents might not work properly everywhere.
*/

use dioxus::core::*;
use dioxus::prelude::*;

fn main() {
    use DomEdit::*;

    let edits = vec![
        // create a container and push it onto the stack
        CreateElement {
            tag: "div",
            root: 0,
        },
        // create an element and push it onto the stack
        CreateElement { tag: "h1", root: 2 },
        // create a text node and push it onto the stack
        CreateTextNode {
            text: "hello world",
            root: 3,
        },
        // append the text node to the h1 element
        AppendChildren { many: 1 },
        // append the h1 element to the container
        AppendChildren { many: 1 },
        // append the container to the default render element ("dioxusroot" if used with default config)
        AppendChildren { many: 1 },
    ];

    dioxus_desktop::run(APP, (), |c| c.with_edits(edits));
}

const APP: FC<()> = |(cx, _props)| {
    rsx!(cx, div {
        "some app"
    })
};
