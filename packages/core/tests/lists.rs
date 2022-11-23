use dioxus::core::Mutation::*;
use dioxus::prelude::*;
use dioxus_core::ElementId;

// A real-world usecase of templates at peak performance
// In react, this would be a lot of node creation.
//
// In Dioxus, we memoize the rsx! body and simplify it down to a few template loads
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            (0..3).map(|i| rsx! {
                div {
                    h1 { "hello world! "}
                    p { "{i}" }
                }
            })
        }
    })
}

#[test]
fn list_renders() {
    let mut dom = VirtualDom::new(app);

    let edits = dom.rebuild().santize();

    assert_eq!(
        edits.edits,
        [
            // Load the outer div
            LoadTemplate { name: "template", index: 0 },
            // Load each template one-by-one, rehydrating it
            LoadTemplate { name: "template", index: 0 },
            HydrateText { path: &[1, 0], value: "0", id: ElementId(6) },
            LoadTemplate { name: "template", index: 0 },
            HydrateText { path: &[1, 0], value: "1", id: ElementId(7) },
            LoadTemplate { name: "template", index: 0 },
            HydrateText { path: &[1, 0], value: "2", id: ElementId(8) },
            // Replace the 0th childn on the div with the 3 templates on the stack
            ReplacePlaceholder { m: 3, path: &[0] },
            // Append the container div to the dom
            AppendChildren { m: 1 }
        ]
    )
}
