use dioxus::dioxus_core::{ElementId, Mutation};
use dioxus::prelude::*;

fn basic_syntax_is_a_template() -> Element {
    let asd = 123;
    let var = 123;

    rsx! {
        div { key: "12345", class: "asd", class: "{asd}", class: if true {
                "{asd}"
            }, class: if false {
                "{asd}"
            }, onclick: move |_| {},
            div { "{var}" }
            div {
                h1 { "var" }
                p { "you're great!" }
                div { background_color: "red",
                    h1 { "var" }
                    div {
                        b { "asd" }
                        "not great"
                    }
                }
                p { "you're great!" }
            }
        }
    }
}

#[test]
fn dual_stream() {
    let mut dom = VirtualDom::new(basic_syntax_is_a_template);
    let edits = dom.rebuild_to_vec().santize();

    use Mutation::*;
    assert_eq!(edits.edits, {
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            SetAttribute {
                name: "class",
                value: "asd 123 123".into_value(),
                id: ElementId(1),
                ns: None,
            },
            NewEventListener { name: "click".to_string(), id: ElementId(1) },
            HydrateText { path: &[0, 0], value: "123".to_string(), id: ElementId(2) },
            AppendChildren { id: ElementId(0), m: 1 },
        ]
    });
}
