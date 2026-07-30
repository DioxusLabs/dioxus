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

    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(1, 1));
    assert_eq!(summary.loads, 1);
    assert_eq!(summary.replaces, 1);

    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(2, 1));
    assert_eq!(summary.loads, 1);
    assert_eq!(summary.replaces, 0);

    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(3, 1));
    assert_eq!(summary.loads, 1);
    assert_eq!(summary.replaces, 0);

    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(4, 1));
    assert_eq!(summary.loads, 1);
    assert_eq!(summary.replaces, 0);
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

    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(2, 1));
    assert_eq!(summary.removes, 1);
    assert_eq!(summary.replaces, 0);

    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(1, 1));
    assert_eq!(summary.removes, 1);
    assert_eq!(summary.replaces, 0);

    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(0, 1));
    assert_eq!(summary.removes, 0);
    assert_eq!(summary.replaces, 1);

    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(3, 1));
    assert_eq!(summary.loads, 3);
    assert_eq!(summary.replaces, 1);
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

    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(1, 2));
    assert_eq!(summary.loads, 2);
    assert_eq!(summary.replaces, 1);

    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(2, 2));
    assert_eq!(summary.loads, 2);
    assert_eq!(summary.replaces, 0);

    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(3, 2));
    assert_eq!(summary.loads, 2);
    assert_eq!(summary.replaces, 0);
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

    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(2, 2));
    assert_eq!(summary.removes, 2);
    assert_eq!(summary.replaces, 0);

    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(1, 2));
    assert_eq!(summary.removes, 2);
    assert_eq!(summary.replaces, 0);

    let summary = rerender(&mut dom, &mut oracle, numbered_outer_div(0, 2));
    assert_eq!(summary.removes, 1);
    assert_eq!(summary.replaces, 1);
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

    let (mut dom, mut oracle, rebuild) = rebuild(app, Vec::new());
    assert_eq!(rebuild, EditSummary::default());

    let summary = rerender(&mut dom, &mut oracle, hello_divs(1));
    assert_eq!(summary.loads, 1);
    assert_eq!(summary.replaces, 1);

    let summary = rerender(&mut dom, &mut oracle, hello_divs(5));
    assert_eq!(summary.loads, 4);
    assert_eq!(summary.replaces, 0);

    let summary = rerender(&mut dom, &mut oracle, Vec::new());
    assert_eq!(summary.removes, 4);
    assert_eq!(summary.replaces, 1);

    let summary = rerender(&mut dom, &mut oracle, hello_divs(1));
    assert_eq!(summary.loads, 1);
    assert_eq!(summary.replaces, 1);
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

    let (mut dom, mut oracle, rebuild) = rebuild(app, vec![ul(Vec::new())]);
    assert_eq!(rebuild.loads, 1);

    let summary = rerender(&mut dom, &mut oracle, vec![ul(fizz_items(1))]);
    assert_eq!(summary.loads, 1);
    assert_eq!(summary.replaces, 1);

    let summary = rerender(&mut dom, &mut oracle, vec![ul(Vec::new())]);
    assert_eq!(summary.loads, 0);
    assert_eq!(summary.replaces, 1);

    let summary = rerender(&mut dom, &mut oracle, vec![ul(fizz_items(3))]);
    assert_eq!(summary.loads, 3);
    assert_eq!(summary.replaces, 2);
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
    vec![div(numbered_children(count, copies))]
}

fn numbered_children(count: usize, copies: usize) -> Vec<SnapshotNode> {
    let mut children = Vec::new();
    for i in 0..count {
        for _ in 0..copies {
            children.push(div(vec![text(i.to_string())]));
        }
    }
    children
}

fn repeated_text_divs(value: &str, count: usize) -> Vec<SnapshotNode> {
    (0..count).map(|_| div(vec![text(value)])).collect()
}

fn hello_divs(count: usize) -> Vec<SnapshotNode> {
    (0..count)
        .map(|i| div(vec![text(format!("hello {i}"))]))
        .collect()
}

fn fizz_items(count: usize) -> Vec<SnapshotNode> {
    (0..count).map(|_| li(vec![text("Fizz")])).collect()
}

fn paragraphs(lines: &[&str]) -> Vec<SnapshotNode> {
    lines.iter().map(|line| p(vec![text(*line)])).collect()
}

fn div(children: Vec<SnapshotNode>) -> SnapshotNode {
    element("div", children)
}

fn ul(children: Vec<SnapshotNode>) -> SnapshotNode {
    element("ul", children)
}

fn li(children: Vec<SnapshotNode>) -> SnapshotNode {
    element("li", children)
}

fn p(children: Vec<SnapshotNode>) -> SnapshotNode {
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
