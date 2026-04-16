use std::{cell::RefCell, time::Duration};

use dioxus::prelude::*;
use dioxus_core::{CapturedError, NoOpMutations};

thread_local! {
    static ACTION: RefCell<Option<Action<(bool,), String>>> = const { RefCell::new(None) };
}

fn app() -> Element {
    let action = use_action(|should_error: bool| async move {
        tokio::time::sleep(Duration::from_millis(10)).await;

        if should_error {
            Err(CapturedError::from_display("boom"))
        } else {
            Ok("done".to_string())
        }
    });

    use_hook(move || {
        ACTION.with(|slot| {
            *slot.borrow_mut() = Some(action);
        });
    });

    _ = action.pending();
    _ = action.result();

    rsx! {}
}

#[tokio::test]
async fn action_projects_ok_and_err_results() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    let mut action = ACTION.with(|slot| slot.borrow().unwrap());

    dom.in_scope(ScopeId::APP, || {
        assert!(action.result().is_none());
        assert!(action.value().is_none());
    });

    dom.in_scope(ScopeId::APP, || {
        action.call(false);
        assert!(action.pending());
        assert!(action.result().is_none());
    });

    tokio::select! {
        _ = async {
            while dom.in_scope(ScopeId::APP, || action.pending()) {
                dom.wait_for_work().await;
                dom.render_immediate(&mut NoOpMutations);
            }
        } => {}
        _ = tokio::time::sleep(Duration::from_millis(500)) => panic!("timed out waiting for successful action")
    }

    dom.in_scope(ScopeId::APP, || {
        let ok = action.result().unwrap().unwrap();
        assert_eq!(&*ok.read(), "done");

        let ok_value = action.value().unwrap().unwrap();
        assert_eq!(&*ok_value.read(), "done");
    });

    dom.in_scope(ScopeId::APP, || {
        action.call(true);
        assert!(action.pending());
        assert!(action.result().is_none());
    });

    tokio::select! {
        _ = async {
            while dom.in_scope(ScopeId::APP, || action.pending()) {
                dom.wait_for_work().await;
                dom.render_immediate(&mut NoOpMutations);
            }
        } => {}
        _ = tokio::time::sleep(Duration::from_millis(500)) => panic!("timed out waiting for errored action")
    }

    dom.in_scope(ScopeId::APP, || {
        let err = action.result().unwrap().unwrap_err();
        assert_eq!((*err.read()).to_string(), "boom");

        let err_value = action.value().unwrap().unwrap_err();
        assert_eq!(err_value.to_string(), "boom");

        action.reset();
        assert!(!action.pending());
        assert!(action.result().is_none());
        assert!(action.value().is_none());
    });
}
