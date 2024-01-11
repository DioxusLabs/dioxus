use dioxus::core::{ElementId, Mutation::*};
use dioxus::prelude::*;

/// When returning sets of components, we do a light diff of the contents to preserve some react-like functionality
///
/// This means that nav_bar should never get re-created and that we should only be swapping out
/// different pointers
#[test]
fn component_swap() {
    fn app(cx: Scope) -> Element {
        let render_phase = cx.use_hook(|| 0);

        *render_phase += 1;

        cx.render(match *render_phase {
            0 => rsx! {
                nav_bar {}
                dash_board {}
            },
            1 => rsx! {
                nav_bar {}
                dash_results {}
            },
            2 => rsx! {
                nav_bar {}
                dash_board {}
            },
            3 => rsx! {
                nav_bar {}
                dash_results {}
            },
            4 => rsx! {
                nav_bar {}
                dash_board {}
            },
            _ => rsx!("blah"),
        })
    }

    fn nav_bar(cx: Scope) -> Element {
        cx.render(rsx! {
            h1 {
                "NavBar"
                {(0..3).map(|_| rsx!(nav_link {}))}
            }
        })
    }

    fn nav_link(cx: Scope) -> Element {
        cx.render(rsx!( h1 { "nav_link" } ))
    }

    fn dash_board(cx: Scope) -> Element {
        cx.render(rsx!( div { "dashboard" } ))
    }

    fn dash_results(cx: Scope) -> Element {
        cx.render(rsx!( div { "results" } ))
    }

    let mut dom = VirtualDom::new(app);
    {
        let edits = dom.rebuild().santize();
        assert_eq!(
            edits.edits,
            [
                LoadTemplate { name: "template", index: 0, id: ElementId(1) },
                LoadTemplate { name: "template", index: 0, id: ElementId(2) },
                LoadTemplate { name: "template", index: 0, id: ElementId(3) },
                LoadTemplate { name: "template", index: 0, id: ElementId(4) },
                ReplacePlaceholder { path: &[1], m: 3 },
                LoadTemplate { name: "template", index: 0, id: ElementId(5) },
                AppendChildren { m: 2, id: ElementId(0) }
            ]
        );
    }

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(6) },
            ReplaceWith { id: ElementId(5), m: 1 }
        ]
    );

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(5) },
            ReplaceWith { id: ElementId(6), m: 1 }
        ]
    );

    dom.mark_dirty(ScopeId::ROOT);
    assert_eq!(
        dom.render_immediate().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(6) },
            ReplaceWith { id: ElementId(5), m: 1 }
        ]
    );
}
