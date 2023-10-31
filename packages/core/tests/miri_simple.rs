use dioxus::prelude::*;

#[test]
fn app_drops() {
    #[component]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            div {}
        })
    }

    let mut dom = VirtualDom::new(App);

    _ = dom.rebuild();
    dom.mark_dirty(ScopeId::ROOT);
    _ = dom.render_immediate();
}

#[test]
fn hooks_drop() {
    #[component]
    fn App(cx: Scope) -> Element {
        cx.use_hook(|| String::from("asd"));
        cx.use_hook(|| String::from("asd"));
        cx.use_hook(|| String::from("asd"));
        cx.use_hook(|| String::from("asd"));

        cx.render(rsx! {
            div {}
        })
    }

    let mut dom = VirtualDom::new(App);

    _ = dom.rebuild();
    dom.mark_dirty(ScopeId::ROOT);
    _ = dom.render_immediate();
}

#[test]
fn contexts_drop() {
    #[component]
    fn App(cx: Scope) -> Element {
        cx.provide_context(String::from("asd"));

        cx.render(rsx! {
            div {
                ChildComp {}
            }
        })
    }

    #[component]
    fn ChildComp(cx: Scope) -> Element {
        let el = cx.consume_context::<String>().unwrap();

        cx.render(rsx! {
            div { "hello {el}" }
        })
    }

    let mut dom = VirtualDom::new(App);

    _ = dom.rebuild();
    dom.mark_dirty(ScopeId::ROOT);
    _ = dom.render_immediate();
}

#[test]
fn tasks_drop() {
    #[component]
    fn App(cx: Scope) -> Element {
        cx.spawn(async {
            // tokio::time::sleep(std::time::Duration::from_millis(100000)).await;
        });

        cx.render(rsx! {
            div { }
        })
    }

    let mut dom = VirtualDom::new(App);

    _ = dom.rebuild();
    dom.mark_dirty(ScopeId::ROOT);
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
    dom.mark_dirty(ScopeId::ROOT);
    _ = dom.render_immediate();
}

#[test]
fn diffing_drops_old() {
    #[component]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            div {
                match cx.generation() % 2 {
                    0 => rsx!( ChildComp1 { name: "asdasd".to_string() }),
                    1 => rsx!( ChildComp2 { name: "asdasd".to_string() }),
                    _ => todo!()
                }
            }
        })
    }

    #[component]
    fn ChildComp1(cx: Scope, name: String) -> Element {
        cx.render(rsx! { "Hello {name}" })
    }

    #[component]
    fn ChildComp2(cx: Scope, name: String) -> Element {
        cx.render(rsx! { "Goodbye {name}"  })
    }

    let mut dom = VirtualDom::new(App);
    _ = dom.rebuild();
    dom.mark_dirty(ScopeId::ROOT);

    _ = dom.render_immediate();
}
