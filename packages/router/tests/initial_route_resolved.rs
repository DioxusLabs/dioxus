use dioxus::prelude::*;
use dioxus_router::OnInitialRouteResolved;
use std::{cell::Cell, rc::Rc};

#[derive(Clone, PartialEq, Routable)]
enum Route {
    #[route("/")]
    Home {},
}

#[component]
fn Home() -> Element {
    rsx! { "home" }
}

fn app() -> Element {
    rsx! { Router::<Route> {} }
}

#[test]
fn initial_route_without_callback() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    assert_eq!(dioxus_ssr::render(&dom), "home");
}

#[test]
fn initial_route_callback_runs_once_with_context() {
    let calls = Rc::new(Cell::new(0));
    let mut dom = VirtualDom::new(app);
    dom.provide_root_context(42usize);
    dom.provide_root_context(OnInitialRouteResolved::new({
        let calls = calls.clone();
        move || {
            assert_eq!(try_consume_context::<usize>(), Some(42));
            calls.set(calls.get() + 1);
        }
    }));
    dom.rebuild_in_place();
    assert_eq!(calls.get(), 1);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut dioxus_vdom::NoOpMutations);
    assert_eq!(calls.get(), 1);
}

#[tokio::test]
async fn initial_route_callback_waits_for_suspense() {
    fn suspended_app() -> Element {
        rsx! {
            SuspenseBoundary {
                fallback: |_| rsx! { "loading" },
                Router::<Route> {}
                Pending {}
            }
        }
    }

    #[component]
    fn Pending() -> Element {
        let ready = use_resource(|| async {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        });
        ready.suspend()?;
        rsx! { "ready" }
    }

    let calls = Rc::new(Cell::new(0));
    let mut dom = VirtualDom::new(suspended_app);
    dom.provide_root_context(42usize);
    dom.provide_root_context(OnInitialRouteResolved::new({
        let calls = calls.clone();
        move || {
            assert_eq!(try_consume_context::<usize>(), Some(42));
            calls.set(calls.get() + 1);
        }
    }));
    dom.rebuild_in_place();
    assert_eq!(calls.get(), 0);
    dom.wait_for_suspense().await;
    assert_eq!(calls.get(), 1);
}
