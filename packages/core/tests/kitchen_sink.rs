use dioxus::core::{ElementId, Mutation};
use dioxus::prelude::*;

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
#[test]
fn dual_stream() {
    let mut dom = VirtualDom::new(basic_syntax_is_a_template);
    let edits = dom.rebuild().santize();

    use Mutation::*;
    assert_eq!(
        edits.template_mutations,
        [
            CreateElement { name: "div" },
            SetStaticAttribute { name: "class", value: "asd", ns: None },
            CreateElement { name: "div" },
            CreateTextPlaceholder,
            AppendChildren { m: 1 },
            CreateElement { name: "div" },
            CreateElement { name: "h1" },
            CreateStaticText { value: "var" },
            AppendChildren { m: 1 },
            CreateElement { name: "p" },
            CreateStaticText { value: "you're great!" },
            AppendChildren { m: 1 },
            CreateElement { name: "div" },
            SetStaticAttribute { name: "background-color", value: "red", ns: Some("style") },
            CreateElement { name: "h1" },
            CreateStaticText { value: "var" },
            AppendChildren { m: 1 },
            CreateElement { name: "div" },
            CreateElement { name: "b" },
            CreateStaticText { value: "asd" },
            AppendChildren { m: 1 },
            CreateStaticText { value: "not great" },
            AppendChildren { m: 2 },
            AppendChildren { m: 2 },
            CreateElement { name: "p" },
            CreateStaticText { value: "you're great!" },
            AppendChildren { m: 1 },
            AppendChildren { m: 4 },
            AppendChildren { m: 2 },
            SaveTemplate { name: "template", m: 1 }
        ],
    );

    assert_eq!(
        edits.edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            SetAttribute { name: "class", value: "123", id: ElementId(1), ns: None },
            NewEventListener { event_name: "click", scope: ScopeId(0), id: ElementId(1) },
            HydrateText { path: &[0, 0], value: "123", id: ElementId(2) },
            AppendChildren { m: 1 }
        ],
    );
}
