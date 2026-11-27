//! Tasks spawned with `Runtime::spawn(scope, ..)` are owned by `scope`, exactly like `spawn` called
//! inside it: unmounting the scope drops them before they are polled again.

use std::cell::Cell;

use dioxus_core::prelude::*;
use dioxus_vdom::{NoOpMutations, Runtime, Task, VirtualDom, current_scope_id, generation};

thread_local! {
    static CHILD: Cell<Option<ScopeId>> = const { Cell::new(None) };
    static POLLED: Cell<bool> = const { Cell::new(false) };
    static DROPPED: Cell<bool> = const { Cell::new(false) };
}

fn app() -> Element {
    if generation() == 0 {
        rsx! { Child {} }
    } else {
        rsx! {}
    }
}

#[component]
fn Child() -> Element {
    CHILD.with(|c| c.set(Some(current_scope_id())));
    rsx! {}
}

struct DropProbe;

impl Drop for DropProbe {
    fn drop(&mut self) {
        DROPPED.with(|d| d.set(true));
    }
}

fn assert_dropped_with_scope(spawn: fn(&Runtime, ScopeId, DropProbe) -> Task) {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    let child = CHILD.with(Cell::get).expect("Child is mounted");

    // The task is queued but not yet polled when its scope is removed in the same frame.
    let task = dom.in_runtime(|| spawn(&Runtime::current(), child, DropProbe));
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);

    assert!(
        DROPPED.with(Cell::get),
        "{task:?} outlived its scope {child:?}"
    );
    assert!(
        !POLLED.with(Cell::get),
        "{task:?} was polled after its scope was removed"
    );
}

async fn pending_with(probe: DropProbe) {
    let _probe = probe;
    POLLED.with(|p| p.set(true));
    std::future::pending::<()>().await;
}

#[test]
fn runtime_spawn_is_dropped_with_its_scope() {
    assert_dropped_with_scope(|rt, scope, probe| rt.spawn(scope, pending_with(probe)));
}

#[test]
fn runtime_spawn_isomorphic_is_dropped_with_its_scope() {
    assert_dropped_with_scope(|rt, scope, probe| rt.spawn_isomorphic(scope, pending_with(probe)));
}
