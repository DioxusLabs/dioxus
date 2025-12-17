use std::cell::RefCell;

use dioxus::prelude::*;
use dioxus_core::{CapturedError, ErrorContext, RenderRedirect, ScopeId, VirtualDom};

thread_local! {
    static INNER_BOUNDARY: RefCell<Option<ErrorContext>> = const { RefCell::new(None) };
}

#[component]
fn InnerBoundary(children: Element) -> Element {
    // Provide an error boundary context in this scope so we can test where errors are inserted.
    // We intentionally don't use `use_error_boundary_provider` here since it's not part of the
    // public `dioxus-core` API surface.
    let ctx = use_hook(|| provide_context(ErrorContext::new(None)));
    use_hook(|| {
        INNER_BOUNDARY.with(|cell| {
            *cell.borrow_mut() = Some(ctx.clone());
        });
    });

    rsx! { {children} }
}

#[component]
fn ThrowRedirect() -> Element {
    Err(CapturedError::new(RenderRedirect::new(307, "/sign-up")).into())
}

fn app() -> Element {
    rsx! {
        InnerBoundary {
            ThrowRedirect {}
        }
    }
}

#[test]
fn render_redirect_bubbles_to_root_error_boundary() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    let inner_ctx = INNER_BOUNDARY.with(|cell| cell.borrow().clone());
    let inner_ctx = inner_ctx.expect("InnerBoundary should have stored its ErrorContext");
    assert!(
        inner_ctx.error().is_none(),
        "RenderRedirect should not be inserted into the nearest user boundary"
    );

    let root_ctx: ErrorContext = dom
        .runtime()
        .consume_context_from_scope(ScopeId::ROOT_ERROR_BOUNDARY)
        .expect("Root error boundary should exist");

    let root_err = root_ctx
        .error()
        .expect("Root boundary should have an error");
    assert!(
        root_err.downcast_ref::<RenderRedirect>().is_some(),
        "Expected RenderRedirect to be thrown into the root error boundary"
    );
}
