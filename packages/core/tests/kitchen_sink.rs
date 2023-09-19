use bumpalo::Bump;
use dioxus::core::{ElementId, Mutation};
use dioxus::prelude::*;

fn basic_syntax_is_a_template(cx: Scope) -> Element {
    let asd = 123;
    let var = 123;

    cx.render(rsx! {
        div { key: "12345",
            class: "asd",
            class: "{asd}",
            class: if true { "{asd}" },
            class: if false { "{asd}" },
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

#[test]
fn dual_stream() {
    let mut dom = VirtualDom::new(basic_syntax_is_a_template);
    let bump = Bump::new();
    let edits = dom.rebuild().santize();

    use Mutation::*;
    assert_eq!(edits.edits, {
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            SetAttribute {
                name: "class",
                value: (&*bump.alloc("asd 123 123".into_value(&bump))).into(),
                id: ElementId(1),
                ns: None,
            },
            NewEventListener { name: "click", id: ElementId(1) },
            HydrateText { path: &[0, 0], value: "123", id: ElementId(2) },
            AppendChildren { id: ElementId(0), m: 1 },
        ]
    });
}
