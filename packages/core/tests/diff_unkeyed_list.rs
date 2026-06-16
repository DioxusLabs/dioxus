use dioxus::prelude::*;
use dioxus_core::generation;
use dioxus_renderer_oracle::{EditSummary, RendererOracle, SnapshotNode};

#[test]
fn list_creates_one_by_one() {
    fn app() -> Element {
        let g = generation();

        rsx! {
            div {
                for i in 0..g {
                    div { "{i}" }
                }
            }
        }
    }

    let (mut dom, mut oracle, rebuild) = rebuild(app, numbered_outer_div(0, 1));
    assert_eq!(rebuild.loads, 1);

    // Anchor diff: each `div { "{i}" }` is one load (the wrapper) + one
    // create_text_node + inserts. No `replace_node_with` because the
    // markerless model uses slot anchors, not placeholder swaps.
    for next in 1..=4 {
        let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(next, 1));
        assert_eq!(summary.loads, 1);
        assert_eq!(summary.create_texts, 1);
        assert_eq!(summary.replaces, 0);
    }
}

#[test]
fn removes_one_by_one() {
    fn app() -> Element {
        let g = 3 - generation() % 4;

        rsx! {
            div {
                for i in 0..g {
                    div { "{i}" }
                }
            }
        }
    }

    let (mut dom, mut oracle, rebuild) = rebuild(app, numbered_outer_div(3, 1));
    assert_eq!(rebuild.loads, 4);

    // Anchor diff: shrinking emits a single `remove_node`. Going back to a
    // populated list inserts fresh templates at the slot's logical anchor
    // (no `replace_node_with`).
    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(2, 1));
    assert_eq!(summary.removes, 1);
    assert_eq!(summary.replaces, 0);

    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(1, 1));
    assert_eq!(summary.removes, 1);
    assert_eq!(summary.replaces, 0);

    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(0, 1));
    assert_eq!(summary.removes, 1);
    assert_eq!(summary.replaces, 0);

    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(3, 1));
    assert_eq!(summary.loads, 3);
    assert_eq!(summary.replaces, 0);
}

#[test]
fn list_shrink_multiroot() {
    fn app() -> Element {
        rsx! {
            div {
                for i in 0..generation() {
                    div { "{i}" }
                    div { "{i}" }
                }
            }
        }
    }

    let (mut dom, mut oracle, rebuild) = rebuild(app, numbered_outer_div(0, 2));
    assert_eq!(rebuild.loads, 1);

    // Two-root iteration: each grown iteration loads 2 templates (the two
    // sibling divs) and creates 2 text nodes for `{i}`.
    for next in 1..=3 {
        let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(next, 2));
        assert_eq!(summary.loads, 2);
        assert_eq!(summary.create_texts, 2);
        assert_eq!(summary.replaces, 0);
    }
}

#[test]
fn removes_one_by_one_multiroot() {
    fn app() -> Element {
        let g = 3 - generation() % 4;

        rsx! {
            div {
                {(0..g).map(|i| rsx! {
                    div { "{i}" }
                    div { "{i}" }
                })}
            }
        }
    }

    let (mut dom, mut oracle, rebuild) = rebuild(app, numbered_outer_div(3, 2));
    assert_eq!(rebuild.loads, 7);

    // Each shrink removes one pair of sibling divs. The final shrink to 0
    // items also removes a pair (no placeholder needs to take their place).
    for next in [2usize, 1, 0] {
        let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(next, 2));
        assert_eq!(summary.removes, 2);
        assert_eq!(summary.replaces, 0);
    }
}

#[test]
fn two_equal_fragments_are_equal_static() {
    fn app() -> Element {
        rsx! {
            for _ in 0..5 {
                div { "hello" }
            }
        }
    }

    let (mut dom, mut oracle, _) = rebuild(app, repeated_text_divs("hello", 5));
    let summary = rerender(&mut dom, &mut oracle, repeated_text_divs("hello", 5));
    assert_eq!(summary, EditSummary::default());
}

#[test]
fn two_equal_fragments_are_equal() {
    fn app() -> Element {
        rsx! {
            for i in 0..5 {
                div { "hello {i}" }
            }
        }
    }

    let (mut dom, mut oracle, _) = rebuild(app, hello_divs(5));
    let summary = rerender(&mut dom, &mut oracle, hello_divs(5));
    assert_eq!(summary, EditSummary::default());
}

#[test]
fn remove_many() {
    fn app() -> Element {
        let num = match generation() % 3 {
            0 => 0,
            1 => 1,
            2 => 5,
            _ => unreachable!(),
        };

        rsx! {
            for i in 0..num {
                div { "hello {i}" }
            }
        }
    }

    // Empty rebuild still pushes the root anchor (`inserts: 1`); no
    // template loads or text creations.
    let (mut dom, mut oracle, rebuild) = rebuild(app, Vec::new());
    assert_eq!(rebuild.loads, 0);
    assert_eq!(rebuild.create_texts, 0);
    assert_eq!(rebuild.removes, 0);
    assert_eq!(rebuild.replaces, 0);

    // 0 -> 1 just inserts a template at the slot anchor.
    let summary = rerender(&mut dom, &mut oracle, hello_divs(1));
    assert_eq!(summary.loads, 1);
    assert_eq!(summary.replaces, 0);

    // 1 -> 5: four new templates loaded after the existing first one.
    let summary = rerender(&mut dom, &mut oracle, hello_divs(5));
    assert_eq!(summary.loads, 4);
    assert_eq!(summary.replaces, 0);

    // 5 -> 0: five `remove_node` calls, no placeholder swap.
    let summary = rerender(&mut dom, &mut oracle, Vec::new());
    assert_eq!(summary.removes, 5);
    assert_eq!(summary.replaces, 0);

    // 0 -> 1 again: still goes through the slot anchor, no replace.
    let summary = rerender(&mut dom, &mut oracle, hello_divs(1));
    assert_eq!(summary.loads, 1);
    assert_eq!(summary.replaces, 0);
}

#[test]
fn replace_and_add_items() {
    fn app() -> Element {
        let items = (0..generation()).map(|_| {
            if generation() % 2 == 0 {
                VNode::empty()
            } else {
                rsx! {
                    li {
                        "Fizz"
                    }
                }
            }
        });

        rsx! {
            ul {
                {items}
            }
        }
    }

    let (mut dom, mut oracle, rebuild) = rebuild(app, vec![snapshot_ul(Vec::new())]);
    assert_eq!(rebuild.loads, 1);

    // 0 (empty) -> 1 fizz: one load, no replace.
    let summary = rerender(&mut dom, &mut oracle, vec![snapshot_ul(fizz_items(1))]);
    assert_eq!(summary.loads, 1);
    assert_eq!(summary.replaces, 0);

    // 1 fizz -> 0 empty: one remove, no replace. The slot stays addressable
    // via the parent's logical anchor.
    let summary = rerender(&mut dom, &mut oracle, vec![snapshot_ul(Vec::new())]);
    assert_eq!(summary.removes, 1);
    assert_eq!(summary.replaces, 0);

    // 0 -> 3 fizzes: three loads inserted at the slot anchor.
    let summary = rerender(&mut dom, &mut oracle, vec![snapshot_ul(fizz_items(3))]);
    assert_eq!(summary.loads, 3);
    assert_eq!(summary.replaces, 0);
}

// Simplified regression test for https://github.com/DioxusLabs/dioxus/issues/4924
#[test]
fn nested_unkeyed_lists() {
    fn app() -> Element {
        let content = if generation() % 2 == 0 {
            vec!["5\n6"]
        } else {
            vec!["1\n2", "3\n4"]
        };

        rsx! {
            for one in &content {
                for line in one.lines() {
                    p { "{line}" }
                }
            }
        }
    }

    let (mut dom, mut oracle, rebuild) = rebuild(app, paragraphs(&["5", "6"]));
    assert_eq!(rebuild.loads, 2);

    let summary = rerender(&mut dom, &mut oracle, paragraphs(&["1", "2", "3", "4"]));
    assert_eq!(summary.loads, 2);
    assert_eq!(summary.set_texts, 2);
}

fn rebuild(
    app: fn() -> Element,
    expected: Vec<SnapshotNode>,
) -> (VirtualDom, RendererOracle, EditSummary) {
    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    let summary = oracle.rebuild(&mut dom);
    assert_eq!(oracle.snapshot(), expected);
    (dom, oracle, summary)
}

fn rerender(
    dom: &mut VirtualDom,
    oracle: &mut RendererOracle,
    expected: Vec<SnapshotNode>,
) -> EditSummary {
    dom.mark_dirty(ScopeId::APP);
    let summary = oracle.render(dom);
    assert_eq!(oracle.snapshot(), expected);
    summary
}

fn numbered_outer_div(count: usize, copies: usize) -> Vec<SnapshotNode> {
    vec![snapshot_div(numbered_children(count, copies))]
}

fn numbered_children(count: usize, copies: usize) -> Vec<SnapshotNode> {
    let mut children = Vec::new();
    for i in 0..count {
        for _ in 0..copies {
            children.push(snapshot_div(vec![text(i.to_string())]));
        }
    }
    children
}

fn repeated_text_divs(value: &str, count: usize) -> Vec<SnapshotNode> {
    (0..count)
        .map(|_| snapshot_div(vec![text(value)]))
        .collect()
}

fn hello_divs(count: usize) -> Vec<SnapshotNode> {
    (0..count)
        .map(|i| snapshot_div(vec![text(format!("hello {i}"))]))
        .collect()
}

fn fizz_items(count: usize) -> Vec<SnapshotNode> {
    (0..count)
        .map(|_| snapshot_li(vec![text("Fizz")]))
        .collect()
}

fn paragraphs(lines: &[&str]) -> Vec<SnapshotNode> {
    lines
        .iter()
        .map(|line| snapshot_p(vec![text(*line)]))
        .collect()
}

fn snapshot_div(children: Vec<SnapshotNode>) -> SnapshotNode {
    element("div", children)
}

fn snapshot_ul(children: Vec<SnapshotNode>) -> SnapshotNode {
    element("ul", children)
}

fn snapshot_li(children: Vec<SnapshotNode>) -> SnapshotNode {
    element("li", children)
}

fn snapshot_p(children: Vec<SnapshotNode>) -> SnapshotNode {
    element("p", children)
}

fn element(tag: &str, children: Vec<SnapshotNode>) -> SnapshotNode {
    SnapshotNode::Element {
        tag: tag.to_string(),
        namespace: None,
        attrs: Vec::new(),
        listeners: Vec::new(),
        children,
    }
}

fn text(value: impl Into<String>) -> SnapshotNode {
    SnapshotNode::Text(value.into())
}
