#![allow(non_snake_case)]

//! Proves out `children: Vec<Element>` as an alternative to `children: Element`: the exact same
//! call-site syntax (natural rsx composition, `for`/`if` included) should populate either shape,
//! with `for`/`if` roots flattened to however many elements they actually produce.

use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
struct WrapProps {
    children: Element,
}

fn Wrap(props: WrapProps) -> Element {
    rsx! {
        div { id: "wrap", {props.children} }
    }
}

#[derive(Props, Clone, PartialEq)]
struct SplitProps {
    children: Vec<Element>,
}

#[derive(Props, Clone, PartialEq)]
struct OptionalProps {
    children: Option<Element>,
}

/// The `Vec<Element>` detection has to be structural - a fully pathed spelling declares the very
/// same field, and defaulting it to a `VNode` instead of a `Vec` wouldn't compile
#[derive(Props, Clone, PartialEq)]
struct PathedProps {
    children: std::vec::Vec<dioxus::prelude::Element>,
}

fn Pathed(props: PathedProps) -> Element {
    let count = props.children.len();
    rsx! {
        div { id: "pathed", "count": count }
    }
}

fn Optional(props: OptionalProps) -> Element {
    let has_label = props.children.is_some();
    rsx! {
        div {
            id: "optional",
            "has-label": has_label,
            if let Some(children) = props.children {
                {children}
            }
        }
    }
}

fn Split(props: SplitProps) -> Element {
    rsx! {
        div {
            id: "split",
            for child in props.children {
                dd { {child} }
            }
        }
    }
}

fn render(app: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);
    let out = dioxus_ssr::render(&dom);
    println!("{out}");
    out
}

#[test]
fn element_children_unchanged() {
    fn app() -> Element {
        rsx! {
            Wrap {
                "a"
                "b"
            }
        }
    }
    let out = render(app);
    assert!(out.contains('a'));
    assert!(out.contains('b'));
}

#[test]
fn vec_children_static_multiple() {
    fn app() -> Element {
        rsx! {
            Split {
                "one"
                "two"
                "three"
            }
        }
    }
    let out = render(app);
    assert_eq!(out.matches("<dd").count(), 3);
}

#[test]
fn vec_children_single() {
    fn app() -> Element {
        rsx! {
            Split { "only" }
        }
    }
    let out = render(app);
    assert_eq!(out.matches("<dd").count(), 1);
}

#[test]
fn vec_children_for_loop_flattens() {
    fn app() -> Element {
        let items = vec!["x", "y", "z", "w"];
        rsx! {
            Split {
                for item in items {
                    "{item}"
                }
            }
        }
    }
    let out = render(app);
    assert_eq!(out.matches("<dd").count(), 4);
}

#[test]
fn vec_children_mixed_literal_and_for_loop() {
    fn app() -> Element {
        let items = vec!["x", "y"];
        rsx! {
            Split {
                "first"
                for item in items {
                    "{item}"
                }
                "last"
            }
        }
    }
    let out = render(app);
    assert_eq!(out.matches("<dd").count(), 4);
}

#[test]
fn vec_children_if_chain_true_branch() {
    fn app() -> Element {
        let show_extra = true;
        rsx! {
            Split {
                "always"
                if show_extra {
                    "extra"
                }
            }
        }
    }
    let out = render(app);
    assert_eq!(out.matches("<dd").count(), 2);
}

#[test]
fn vec_children_if_chain_false_branch_contributes_zero() {
    fn app() -> Element {
        let show_extra = false;
        rsx! {
            Split {
                "always"
                if show_extra {
                    "extra"
                }
            }
        }
    }
    let out = render(app);
    assert_eq!(out.matches("<dd").count(), 1);
}

#[test]
fn vec_children_nested_for_and_if() {
    fn app() -> Element {
        let groups = vec![vec!["a1", "a2"], vec!["b1"]];
        rsx! {
            Split {
                for group in groups {
                    for item in group {
                        if item != "skip" {
                            "{item}"
                        }
                    }
                }
            }
        }
    }
    let out = render(app);
    assert_eq!(out.matches("<dd").count(), 3);
}

#[test]
fn vec_children_empty_default() {
    fn app() -> Element {
        rsx! {
            Split {}
        }
    }
    let out = render(app);
    assert_eq!(out.matches("<dd").count(), 0);
}

#[test]
fn vec_children_pathed_type_still_defaults_to_empty_vec() {
    fn app() -> Element {
        rsx! {
            Pathed {}
            Pathed {
                span {}
                span {}
            }
        }
    }
    let out = render(app);
    assert!(out.contains("count=0"));
    assert!(out.contains("count=2"));
}

#[test]
fn option_element_children_with_content() {
    fn app() -> Element {
        rsx! {
            Optional { "a label" }
        }
    }
    let out = render(app);
    assert!(out.contains("has-label=true"));
    assert!(out.contains("a label"));
}

#[test]
fn option_element_children_empty_is_none() {
    fn app() -> Element {
        rsx! {
            Optional {}
        }
    }
    let out = render(app);
    assert!(out.contains("has-label=false"));
}
