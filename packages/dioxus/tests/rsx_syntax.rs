fn basic_syntax_is_a_template(cx: Scope) -> Element {
    let asd = 123;
    let var = 123;

    cx.render(rsx! {
        div { key: "12345",
            class: "asd",
            class: "{asd}",
            onclick: move |_| {},
            div { "{var}" }
            div {
                h1 { "var" }
                p { "you're great!" }
                div { background_color: "red",
                    h1 { "var" }
                    div { b { "asd" } "not great" }
                }
                p { "you're great!" }
            }
        }
    })
}

// imports come after the test since the rsx! macro is sensitive to its location in the file
// (the byte index is used to differentiate sub templates)
use dioxus::core::{ElementId, Mutation};
use dioxus::prelude::*;

#[test]
fn dual_stream() {
    let mut dom = VirtualDom::new(basic_syntax_is_a_template);
    let edits = dom.rebuild();

    use Mutation::*;
    assert_eq!(
        edits.template_mutations,
        vec![
            CreateElement { name: "div", namespace: None, id: ElementId(1) },
            SetAttribute { name: "class", value: "asd", id: ElementId(1) },
            CreateElement { name: "div", namespace: None, id: ElementId(2) },
            CreatePlaceholder { id: ElementId(3) },
            AppendChildren { m: 1 },
            CreateElement { name: "div", namespace: None, id: ElementId(4) },
            CreateElement { name: "h1", namespace: None, id: ElementId(5) },
            CreateText { value: "var" },
            AppendChildren { m: 1 },
            CreateElement { name: "p", namespace: None, id: ElementId(6) },
            CreateText { value: "you're great!" },
            AppendChildren { m: 1 },
            CreateElement { name: "div", namespace: None, id: ElementId(7) },
            SetAttribute { name: "background-color", value: "red", id: ElementId(7) },
            CreateElement { name: "h1", namespace: None, id: ElementId(8) },
            CreateText { value: "var" },
            AppendChildren { m: 1 },
            CreateElement { name: "div", namespace: None, id: ElementId(9) },
            CreateElement { name: "b", namespace: None, id: ElementId(10) },
            CreateText { value: "asd" },
            AppendChildren { m: 1 },
            CreateText { value: "not great" },
            AppendChildren { m: 2 },
            AppendChildren { m: 2 },
            CreateElement { name: "p", namespace: None, id: ElementId(11) },
            CreateText { value: "you're great!" },
            AppendChildren { m: 1 },
            AppendChildren { m: 4 },
            AppendChildren { m: 2 },
            SaveTemplate { name: "packages/dioxus/tests/rsx_syntax.rs:5:15:122", m: 1 }
        ]
    );

    assert_eq!(
        edits.edits,
        vec![
            LoadTemplate { name: "packages/dioxus/tests/rsx_syntax.rs:5:15:122", index: 0 },
            AssignId { path: &[], id: ElementId(12) },
            SetAttribute { name: "class", value: "123", id: ElementId(12) },
            SetAttribute { name: "onclick", value: "asd", id: ElementId(12) }, // ---- todo: listeners
            HydrateText { path: &[0, 0], value: "123", id: ElementId(13) },
            ReplacePlaceholder { m: 1, path: &[0, 0] },
            AppendChildren { m: 1 }
        ]
    );
}
