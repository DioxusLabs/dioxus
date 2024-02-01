use dioxus::prelude::*;

#[test]
fn app_drops() {
    fn app() -> Element {
        rsx! { div {} }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild(&mut dioxus_core::NoOpMutations);
    dom.mark_dirty(ScopeId::ROOT);
    _ = dom.render_immediate_to_vec();
}

#[test]
fn hooks_drop() {
    fn app() -> Element {
        use_hook(|| String::from("asd"));
        use_hook(|| String::from("asd"));
        use_hook(|| String::from("asd"));
        use_hook(|| String::from("asd"));

        rsx! { div {} }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild(&mut dioxus_core::NoOpMutations);
    dom.mark_dirty(ScopeId::ROOT);
    _ = dom.render_immediate_to_vec();
}

#[test]
fn contexts_drop() {
    fn app() -> Element {
        provide_context(String::from("asd"));

        rsx! {
            div { ChildComp {} }
        }
    }

    #[allow(non_snake_case)]
    fn ChildComp() -> Element {
        let el = consume_context::<String>();

        rsx! { div { "hello {el}" } }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild(&mut dioxus_core::NoOpMutations);
    dom.mark_dirty(ScopeId::ROOT);
    _ = dom.render_immediate_to_vec();
}

#[test]
fn tasks_drop() {
    fn app() -> Element {
        spawn(async {
            // tokio::time::sleep(std::time::Duration::from_millis(100000)).await;
        });

        rsx! { div {} }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild(&mut dioxus_core::NoOpMutations);
    dom.mark_dirty(ScopeId::ROOT);
    _ = dom.render_immediate_to_vec();
}

#[test]
fn root_props_drop() {
    #[derive(Clone)]
    struct RootProps(String);

    let mut dom = VirtualDom::new_with_props(
        |cx: RootProps| rsx!( div { "{cx.0}" } ),
        RootProps("asdasd".to_string()),
    );

    dom.rebuild(&mut dioxus_core::NoOpMutations);
    dom.mark_dirty(ScopeId::ROOT);
    _ = dom.render_immediate_to_vec();
}

#[test]
fn diffing_drops_old() {
    fn app() -> Element {
        rsx! {
            div {
                match generation() % 2 {
                    0 => rsx!( ChildComp1 { name: "asdasd".to_string() }),
                    1 => rsx!( ChildComp2 { name: "asdasd".to_string() }),
                    _ => unreachable!()
                }
            }
        }
    }

    #[component]
    fn ChildComp1(name: String) -> Element {
        rsx! {"Hello {name}"}
    }

    #[component]
    fn ChildComp2(name: String) -> Element {
        rsx! {"Goodbye {name}"}
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);
    dom.mark_dirty(ScopeId::ROOT);

    _ = dom.render_immediate_to_vec();
}

#[test]
fn hooks_drop_before_contexts() {
    fn app() -> Element {
        provide_context(123i32);
        use_hook(|| {
            #[derive(Clone)]
            struct ReadsContextOnDrop;

            impl Drop for ReadsContextOnDrop {
                fn drop(&mut self) {
                    assert_eq!(123, consume_context::<i32>());
                }
            }

            ReadsContextOnDrop
        });

        rsx! { div {} }
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild(&mut dioxus_core::NoOpMutations);
    dom.mark_dirty(ScopeId::ROOT);
    _ = dom.render_immediate_to_vec();
}
