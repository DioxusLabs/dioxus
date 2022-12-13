use dioxus::prelude::*;

#[test]
fn app_drops() {
    fn app(cx: Scope) -> Element {
        cx.render(rsx! {
            div {}
        })
    }

    let mut dom = VirtualDom::new(app);

    _ = dom.rebuild();
    dom.mark_dirty(ScopeId(0));
    _ = dom.render_immediate();
}

#[test]
fn hooks_drop() {
    fn app(cx: Scope) -> Element {
        cx.use_hook(|| String::from("asd"));
        cx.use_hook(|| String::from("asd"));
        cx.use_hook(|| String::from("asd"));
        cx.use_hook(|| String::from("asd"));

        cx.render(rsx! {
            div {}
        })
    }

    let mut dom = VirtualDom::new(app);

    _ = dom.rebuild();
    dom.mark_dirty(ScopeId(0));
    _ = dom.render_immediate();
}

#[test]
fn contexts_drop() {
    fn app(cx: Scope) -> Element {
        cx.provide_context(String::from("asd"));

        cx.render(rsx! {
            div {
                child_comp {}
            }
        })
    }

    fn child_comp(cx: Scope) -> Element {
        let el = cx.consume_context::<String>().unwrap();

        cx.render(rsx! {
            div { "hello {el}" }
        })
    }

    let mut dom = VirtualDom::new(app);

    _ = dom.rebuild();
    dom.mark_dirty(ScopeId(0));
    _ = dom.render_immediate();
}

#[tokio::test]
fn tasks_drop() {
    fn app(cx: Scope) -> Element {
        cx.spawn(async {
            tokio::time::sleep(std::time::Duration::from_millis(100000)).await;
        });

        cx.render(rsx! {
            div { }
        })
    }

    let mut dom = VirtualDom::new(app);

    _ = dom.rebuild();
    dom.mark_dirty(ScopeId(0));
    _ = dom.render_immediate();
}

#[test]
fn root_props_drop() {
    struct RootProps(String);

    let mut dom = VirtualDom::new_with_props(
        |cx| cx.render(rsx!( div { "{cx.props.0}"  } )),
        RootProps("asdasd".to_string()),
    );

    _ = dom.rebuild();
    dom.mark_dirty(ScopeId(0));
    _ = dom.render_immediate();
}

#[test]
fn diffing_drops_old() {
    fn app(cx: Scope) -> Element {
        cx.render(rsx! {
            div {
                match cx.generation() % 2 {
                    0 => rsx!( child_comp1 { name: "asdasd".to_string() }),
                    1 => rsx!( child_comp2 { name: "asdasd".to_string() }),
                    _ => todo!()
                }
            }
        })
    }

    #[inline_props]
    fn child_comp1(cx: Scope, name: String) -> Element {
        cx.render(rsx! { "Hello {name}" })
    }

    #[inline_props]
    fn child_comp2(cx: Scope, name: String) -> Element {
        cx.render(rsx! { "Goodbye {name}"  })
    }

    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild();
    dom.mark_dirty(ScopeId(0));

    _ = dom.render_immediate();
}
