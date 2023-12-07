#![allow(unused, non_upper_case_globals)]
#![allow(non_snake_case)]

//! Tests for the lifecycle of components.
use dioxus::core::{ElementId, Mutation::*};
use dioxus::prelude::*;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

type Shared<T> = Arc<Mutex<T>>;

#[test]
fn manual_diffing() {
    struct AppProps {
        value: Shared<&'static str>,
    }

    fn app(cx: Scope<AppProps>) -> Element {
        let val = cx.props.value.lock().unwrap();
        cx.render(rsx! { div { "{val}" } })
    };

    let value = Arc::new(Mutex::new("Hello"));
    let mut dom = VirtualDom::new_with_props(app, AppProps { value: value.clone() });

    let _ = dom.rebuild();

    *value.lock().unwrap() = "goodbye";

    assert_eq!(
        dom.rebuild().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(3) },
            HydrateText { path: &[0], value: "goodbye", id: ElementId(4) },
            AppendChildren { m: 1, id: ElementId(0) }
        ]
    );
}

#[test]
fn events_generate() {
    fn app(cx: Scope) -> Element {
        let count = cx.use_hook(|| 0);

        match *count {
            0 => cx.render(rsx! {
                div { onclick: move |_| *count += 1,
                    div { "nested" }
                    "Click me!"
                }
            }),
            _ => cx.render(rsx!(())),
        }
    };

    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild();

    dom.handle_event("click", Rc::new(MouseData::default()), ElementId(1), true);

    dom.mark_dirty(ScopeId::ROOT);
    let edits = dom.render_immediate();

    assert_eq!(
        edits.edits,
        [
            CreatePlaceholder { id: ElementId(2) },
            ReplaceWith { id: ElementId(1), m: 1 }
        ]
    )
}

// #[test]
// fn components_generate() {
//     fn app(cx: Scope) -> Element {
//         let render_phase = cx.use_hook(|| 0);
//         *render_phase += 1;

//         cx.render(match *render_phase {
//             1 => rsx_without_templates!("Text0"),
//             2 => rsx_without_templates!(div {}),
//             3 => rsx_without_templates!("Text2"),
//             4 => rsx_without_templates!(Child {}),
//             5 => rsx_without_templates!({ None as Option<()> }),
//             6 => rsx_without_templates!("text 3"),
//             7 => rsx_without_templates!({ (0..2).map(|f| rsx_without_templates!("text {f}")) }),
//             8 => rsx_without_templates!(Child {}),
//             _ => todo!(),
//         })
//     };

//     fn Child(cx: Scope) -> Element {
//         println!("Running child");
//         cx.render(rsx_without_templates! {
//             h1 {}
//         })
//     }

//     let mut dom = VirtualDom::new(app);
//     let edits = dom.rebuild();
//     assert_eq!(
//         edits.edits,
//         [
//             CreateTextNode { root: Some(1), text: "Text0" },
//             AppendChildren { root: Some(0), children: vec![1] }
//         ]
//     );

//     assert_eq!(
//         dom.hard_diff(ScopeId::ROOT).edits,
//         [
//             CreateElement { root: Some(2), tag: "div", children: 0 },
//             ReplaceWith { root: Some(1), nodes: vec![2] }
//         ]
//     );

//     assert_eq!(
//         dom.hard_diff(ScopeId::ROOT).edits,
//         [
//             CreateTextNode { root: Some(1), text: "Text2" },
//             ReplaceWith { root: Some(2), nodes: vec![1] }
//         ]
//     );

//     // child {}
//     assert_eq!(
//         dom.hard_diff(ScopeId::ROOT).edits,
//         [
//             CreateElement { root: Some(2), tag: "h1", children: 0 },
//             ReplaceWith { root: Some(1), nodes: vec![2] }
//         ]
//     );

//     // placeholder
//     assert_eq!(
//         dom.hard_diff(ScopeId::ROOT).edits,
//         [
//             CreatePlaceholder { root: Some(1) },
//             ReplaceWith { root: Some(2), nodes: vec![1] }
//         ]
//     );

//     assert_eq!(
//         dom.hard_diff(ScopeId::ROOT).edits,
//         [
//             CreateTextNode { root: Some(2), text: "text 3" },
//             ReplaceWith { root: Some(1), nodes: vec![2] }
//         ]
//     );

//     assert_eq!(
//         dom.hard_diff(ScopeId::ROOT).edits,
//         [
//             CreateTextNode { text: "text 0", root: Some(1) },
//             CreateTextNode { text: "text 1", root: Some(3) },
//             ReplaceWith { root: Some(2), nodes: vec![1, 3] },
//         ]
//     );

//     assert_eq!(
//         dom.hard_diff(ScopeId::ROOT).edits,
//         [
//             CreateElement { tag: "h1", root: Some(2), children: 0 },
//             ReplaceWith { root: Some(1), nodes: vec![2] },
//             Remove { root: Some(3) },
//         ]
//     );
// }
