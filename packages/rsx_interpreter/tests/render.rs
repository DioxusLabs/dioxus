use dioxus::prelude::*;

#[test]
#[allow(non_snake_case)]
fn render_basic() {
    fn Base(cx: Scope) -> Element {
        rsx!(cx, div {})
    }

    let dom = VirtualDom::new(Base);
    let static_vnodes = rsx!(div{"hello world"});
    let location = CodeLocation {
        file_path: String::new(),
        crate_path: String::new(),
        line: 0,
        column: 0,
    };
    let empty_context = CapturedContext {
        captured: IfmtArgs {
            named_args: Vec::new(),
        },
        components: Vec::new(),
        iterators: Vec::new(),
        expressions: Vec::new(),
        listeners: Vec::new(),
        location: location.clone(),
    };
    let interperted_vnodes = LazyNodes::new(|factory| {
        dioxus_rsx_interpreter::resolve_scope(
            location,
            "div{\"hello world\"}",
            empty_context,
            factory,
        )
    });

    let interperted_vnodes = dom.render_vnodes(interperted_vnodes);
    let static_vnodes = dom.render_vnodes(static_vnodes);
    assert!(check_eq(interperted_vnodes, static_vnodes));
}

#[test]
#[allow(non_snake_case)]
fn render_nested() {
    fn Base(cx: Scope) -> Element {
        rsx!(cx, div {})
    }

    let dom = VirtualDom::new(Base);
    let static_vnodes = rsx! {
        div {
            p { "hello world" }
            div {
                p { "hello world" }
            }
        }
    };
    let location = CodeLocation {
        file_path: String::new(),
        crate_path: String::new(),
        line: 1,
        column: 0,
    };
    let empty_context = CapturedContext {
        captured: IfmtArgs {
            named_args: Vec::new(),
        },
        components: Vec::new(),
        iterators: Vec::new(),
        expressions: Vec::new(),
        listeners: Vec::new(),
        location: location.clone(),
    };
    let interperted_vnodes = LazyNodes::new(|factory| {
        dioxus_rsx_interpreter::resolve_scope(
            location,
            r#"div {
                p { "hello world" }
                div {
                    p { "hello world" }
                }
            }"#,
            empty_context,
            factory,
        )
    });

    let interperted_vnodes = dom.render_vnodes(interperted_vnodes);
    let static_vnodes = dom.render_vnodes(static_vnodes);
    assert!(check_eq(interperted_vnodes, static_vnodes));
}

#[test]
#[allow(non_snake_case)]
fn render_component() {
    fn Comp(cx: Scope) -> Element {
        rsx!(cx, div {})
    }

    fn Base(cx: Scope) -> Element {
        rsx!(cx, div {})
    }

    let dom = VirtualDom::new(Base);
    let static_vnodes = rsx! {
        div {
            Comp {}
        }
    };
    let location = CodeLocation {
        file_path: String::new(),
        crate_path: String::new(),
        line: 2,
        column: 0,
    };

    let interperted_vnodes = LazyNodes::new(|factory| {
        let context = CapturedContext {
            captured: IfmtArgs {
                named_args: Vec::new(),
            },
            components: vec![(
                r#"__cx.component(Comp, fc_to_builder(Comp).build(), None, "Comp")"#,
                factory.component(Comp, (), None, "Comp"),
            )],
            iterators: Vec::new(),
            expressions: Vec::new(),
            listeners: Vec::new(),
            location: location.clone(),
        };
        dioxus_rsx_interpreter::resolve_scope(
            location,
            r#"div {
                Comp {}
            }"#,
            context,
            factory,
        )
    });

    let interperted_vnodes = dom.render_vnodes(interperted_vnodes);
    let static_vnodes = dom.render_vnodes(static_vnodes);
    println!("{:#?}", interperted_vnodes);
    println!("{:#?}", static_vnodes);
    assert!(check_eq(interperted_vnodes, static_vnodes));
}

#[test]
#[allow(non_snake_case)]
fn render_iterator() {
    fn Base(cx: Scope) -> Element {
        rsx!(cx, div {})
    }

    let dom = VirtualDom::new(Base);
    let iter = (0..10).map(|i| dom.render_vnodes(rsx! {"{i}"}));
    let static_vnodes = rsx! {
        div {
            iter
        }
    };
    let location = CodeLocation {
        file_path: String::new(),
        crate_path: String::new(),
        line: 3,
        column: 0,
    };

    let interperted_vnodes = LazyNodes::new(|factory| {
        let context = CapturedContext {
            captured: IfmtArgs {
                named_args: Vec::new(),
            },
            components: Vec::new(),
            iterators: vec![(
                r#"
            (0..10).map(|i| dom.render_vnodes(rsx!{"{i}"}))"#,
                factory.fragment_from_iter((0..10).map(|i| factory.text(format_args!("{i}")))),
            )],
            expressions: Vec::new(),
            listeners: Vec::new(),
            location: location.clone(),
        };
        dioxus_rsx_interpreter::resolve_scope(
            location,
            r#"div {
                (0..10).map(|i| dom.render_vnodes(rsx!{"{i}"}))
            }"#,
            context,
            factory,
        )
    });

    let interperted_vnodes = dom.render_vnodes(interperted_vnodes);
    let static_vnodes = dom.render_vnodes(static_vnodes);
    println!("{:#?}", interperted_vnodes);
    println!("{:#?}", static_vnodes);
    assert!(check_eq(interperted_vnodes, static_vnodes));
}

#[test]
#[allow(non_snake_case)]
fn render_captured_variable() {
    fn Base(cx: Scope) -> Element {
        rsx!(cx, div {})
    }

    let dom = VirtualDom::new(Base);

    let x = 10;
    let static_vnodes = rsx! {
        div {
            "{x}"
        }
    };
    let location = CodeLocation {
        file_path: String::new(),
        crate_path: String::new(),
        line: 4,
        column: 0,
    };

    let interperted_vnodes = LazyNodes::new(|factory| {
        let context = CapturedContext {
            captured: IfmtArgs {
                named_args: vec![FormattedArg {
                    expr: "x",
                    format_args: "",
                    result: x.to_string(),
                }],
            },
            components: Vec::new(),
            iterators: Vec::new(),
            expressions: Vec::new(),
            listeners: Vec::new(),
            location: location.clone(),
        };
        dioxus_rsx_interpreter::resolve_scope(
            location,
            r#"div {
                "{x}"
            }"#,
            context,
            factory,
        )
    });

    let interperted_vnodes = dom.render_vnodes(interperted_vnodes);
    let static_vnodes = dom.render_vnodes(static_vnodes);
    println!("{:#?}", interperted_vnodes);
    println!("{:#?}", static_vnodes);
    assert!(check_eq(interperted_vnodes, static_vnodes));
}

#[test]
#[allow(non_snake_case)]
fn render_listener() {
    fn Base(cx: Scope) -> Element {
        rsx!(cx, div {})
    }

    let dom = VirtualDom::new(Base);
    let static_vnodes = rsx! {
        div {
            onclick: |_| println!("clicked")
        }
    };
    let location = CodeLocation {
        file_path: String::new(),
        crate_path: String::new(),
        line: 5,
        column: 0,
    };

    let interperted_vnodes = LazyNodes::new(|factory| {
        let f = |_| println!("clicked");
        let f = factory.bump().alloc(f);
        let context = CapturedContext {
            captured: IfmtArgs {
                named_args: Vec::new(),
            },
            components: Vec::new(),
            iterators: Vec::new(),
            expressions: Vec::new(),
            listeners: vec![(
                r#"dioxus_elements::on::onclick(__cx, |_| println!("clicked"))"#,
                dioxus_elements::on::onclick(factory, f),
            )],
            location: location.clone(),
        };
        dioxus_rsx_interpreter::resolve_scope(
            location,
            r#"div {
                onclick: |_| println!("clicked")
            }"#,
            context,
            factory,
        )
    });

    let interperted_vnodes = dom.render_vnodes(interperted_vnodes);
    let static_vnodes = dom.render_vnodes(static_vnodes);
    println!("{:#?}", interperted_vnodes);
    println!("{:#?}", static_vnodes);
    assert!(check_eq(interperted_vnodes, static_vnodes));
}

fn check_eq<'a>(a: &'a VNode<'a>, b: &'a VNode<'a>) -> bool {
    match (a, b) {
        (VNode::Text(t_a), VNode::Text(t_b)) => t_a.text == t_b.text,
        (VNode::Element(e_a), VNode::Element(e_b)) => {
            e_a.attributes
                .iter()
                .zip(e_b.attributes.iter())
                .all(|(a, b)| {
                    a.is_static == b.is_static
                        && a.is_volatile == b.is_volatile
                        && a.name == b.name
                        && a.value == b.value
                        && a.namespace == b.namespace
                })
                && e_a
                    .children
                    .iter()
                    .zip(e_b.children.iter())
                    .all(|(a, b)| check_eq(a, b))
                && e_a.key == e_b.key
                && e_a.tag == e_b.tag
                && e_a.namespace == e_b.namespace
                && e_a
                    .listeners
                    .iter()
                    .zip(e_b.listeners.iter())
                    .all(|(a, b)| a.event == b.event)
        }
        (VNode::Fragment(f_a), VNode::Fragment(f_b)) => {
            f_a.key == f_b.key
                && f_a
                    .children
                    .iter()
                    .zip(f_b.children.iter())
                    .all(|(a, b)| check_eq(a, b))
        }
        (VNode::Component(c_a), VNode::Component(c_b)) => {
            c_a.can_memoize == c_b.can_memoize
                && c_a.key == c_b.key
                && c_a.fn_name == c_b.fn_name
                && c_a.user_fc == c_b.user_fc
        }
        (VNode::Placeholder(_), VNode::Placeholder(_)) => true,
        _ => false,
    }
}
