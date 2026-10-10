use dioxus_core::{
    Callback, CapturedError, ErrorContext, Event, ListenerCallback, NoOpMutations, ScopeId, VNode,
    VirtualDom, current_scope_id, provide_context,
};
use futures_channel::oneshot;
use std::{cell::Cell, rc::Rc};

fn callback_dom() -> (VirtualDom, ErrorContext) {
    let mut dom = VirtualDom::new(VNode::empty);
    dom.rebuild(&mut NoOpMutations);
    let errors = ErrorContext::new(None);
    dom.in_scope(ScopeId::APP, || provide_context(errors.clone()));
    (dom, errors)
}

fn failure(message: &'static str) -> dioxus_core::Result<()> {
    Err(CapturedError::from_display(message))
}

fn assert_error(errors: &ErrorContext, message: &str) {
    assert_eq!(
        errors.error().expect("callback error was lost").to_string(),
        message
    );
}

#[test]
fn ready_callback_error_is_reported_before_returning() {
    let (mut dom, errors) = callback_dom();
    let callback: Callback<()> = dom.in_scope(ScopeId::APP, || {
        Callback::new(|()| async {
            assert_eq!(current_scope_id(), ScopeId::APP);
            failure("ready callback")
        })
    });

    dom.in_scope(ScopeId::ROOT, || callback.call(()));
    assert_error(&errors, "ready callback");
    errors.clear_errors();
    dom.render_immediate(&mut NoOpMutations);
    assert!(
        errors.error().is_none(),
        "ready future must not be spawned again"
    );
}

#[test]
fn ready_listener_error_preserves_synchronous_event_effects() {
    let (mut dom, errors) = callback_dom();
    let listener = dom.in_scope(ScopeId::APP, || {
        ListenerCallback::new(|event: Event<()>| async move {
            assert_eq!(current_scope_id(), ScopeId::APP);
            event.prevent_default();
            event.stop_propagation();
            failure("ready listener")
        })
    });
    let event = Event::new(Rc::new(()), true).into_any();

    dom.in_scope(ScopeId::ROOT, || listener.call(event.clone()));
    assert!(!event.default_action_enabled());
    assert!(!event.propagates());
    assert_error(&errors, "ready listener");
    errors.clear_errors();
    dom.render_immediate(&mut NoOpMutations);
    assert!(errors.error().is_none());
}

#[test]
fn pending_callback_error_is_reported_when_resumed() {
    let (mut dom, errors) = callback_dom();
    let (sender, receiver) = oneshot::channel();
    let mut receiver = Some(receiver);
    let polls = Rc::new(Cell::new(0));
    let callback: Callback<()> = dom.in_scope(ScopeId::APP, || {
        let polls = polls.clone();
        Callback::new(move |()| {
            let receiver = receiver.take().unwrap();
            let polls = polls.clone();
            async move {
                polls.set(polls.get() + 1);
                receiver.await.unwrap();
                assert_eq!(current_scope_id(), ScopeId::APP);
                failure("pending callback")
            }
        })
    });

    callback.call(());
    assert_eq!(polls.get(), 1, "the future must start in the callback tick");
    assert!(errors.error().is_none());
    dom.render_immediate(&mut NoOpMutations);
    assert!(errors.error().is_none());
    sender.send(()).unwrap();
    dom.render_immediate(&mut NoOpMutations);
    assert_eq!(polls.get(), 1, "resuming must not restart the future");
    assert_error(&errors, "pending callback");
}

#[test]
fn pending_listener_error_preserves_event_effects_before_resuming() {
    let (mut dom, errors) = callback_dom();
    let (sender, receiver) = oneshot::channel();
    let mut receiver = Some(receiver);
    let listener = dom.in_scope(ScopeId::APP, || {
        ListenerCallback::new(move |event: Event<()>| {
            let receiver = receiver.take().unwrap();
            async move {
                event.prevent_default();
                receiver.await.unwrap();
                assert_eq!(current_scope_id(), ScopeId::APP);
                event.stop_propagation();
                failure("pending listener")
            }
        })
    });
    let event = Event::new(Rc::new(()), true).into_any();

    dom.in_scope(ScopeId::ROOT, || listener.call(event.clone()));
    assert!(!event.default_action_enabled());
    assert!(event.propagates());
    assert!(errors.error().is_none());
    sender.send(()).unwrap();
    dom.render_immediate(&mut NoOpMutations);
    assert!(!event.propagates());
    assert_error(&errors, "pending listener");
}

#[test]
fn successful_futures_do_not_replace_existing_errors() {
    let (mut dom, errors) = callback_dom();
    errors.insert_error(CapturedError::from_display("existing error"));
    let completed = Rc::new(Cell::new(0));
    let ready: Callback<()> = dom.in_scope(ScopeId::APP, || {
        let completed = completed.clone();
        Callback::new(move |()| {
            let completed = completed.clone();
            async move {
                completed.set(completed.get() + 1);
                Ok::<(), dioxus_core::CapturedError>(())
            }
        })
    });
    let (sender, receiver) = oneshot::channel();
    let mut receiver = Some(receiver);
    let pending: Callback<()> = dom.in_scope(ScopeId::APP, || {
        let completed = completed.clone();
        Callback::new(move |()| {
            let receiver = receiver.take().unwrap();
            let completed = completed.clone();
            async move {
                receiver.await.unwrap();
                completed.set(completed.get() + 1);
                Ok::<(), dioxus_core::CapturedError>(())
            }
        })
    });

    ready.call(());
    pending.call(());
    assert_eq!(completed.get(), 1);
    assert_error(&errors, "existing error");
    sender.send(()).unwrap();
    dom.render_immediate(&mut NoOpMutations);
    assert_eq!(completed.get(), 2);
    assert_error(&errors, "existing error");
    dom.render_immediate(&mut NoOpMutations);
    assert_eq!(completed.get(), 2);
}

#[test]
fn ready_async_error_renders_the_nearest_error_boundary() {
    use dioxus::prelude::*;

    fn app() -> Element {
        rsx! {
            ErrorBoundary {
                handle_error: |errors: ErrorContext| rsx! { "caught: {errors.error().unwrap()}" },
                ThrowsReadyError {}
            }
        }
    }

    #[component]
    fn ThrowsReadyError() -> Element {
        use_hook(|| {
            let callback: Callback<()> = Callback::new(|()| async { failure("visible failure") });
            callback.call(());
        });
        rsx! { "child content" }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut NoOpMutations);
    dom.render_immediate(&mut NoOpMutations);
    assert_eq!(dioxus_ssr::render(&dom), "caught: visible failure");
}
