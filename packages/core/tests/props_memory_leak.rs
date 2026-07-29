//! Regression tests for per-render memory leaks in converted component props.
//! https://github.com/DioxusLabs/dioxus/issues/5671
#![allow(non_snake_case)]

use dioxus::prelude::dioxus_core::NoOpMutations;
use dioxus::prelude::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering};

struct CountingAllocator;

static LIVE_BYTES: AtomicIsize = AtomicIsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LIVE_BYTES.fetch_add(layout.size() as isize, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE_BYTES.fetch_sub(layout.size() as isize, Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        LIVE_BYTES.fetch_add(
            new_size as isize - layout.size() as isize,
            Ordering::Relaxed,
        );
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

const WARMUP: usize = 200;
const ITERS: usize = 1000;

/// Rerender the app many times and return the growth in live heap bytes per render.
fn bytes_leaked_per_render(app: fn() -> Element) -> f64 {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    for _ in 0..WARMUP {
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);
    }
    let start = LIVE_BYTES.load(Ordering::Relaxed);
    for _ in 0..ITERS {
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);
    }
    let end = LIVE_BYTES.load(Ordering::Relaxed);
    (end - start) as f64 / ITERS as f64
}

#[track_caller]
fn assert_no_leak(name: &str, app: fn() -> Element) {
    let per_render = bytes_leaked_per_render(app);
    assert!(
        per_render < 16.0,
        "{name} leaked {per_render} bytes per render"
    );
}

#[component]
fn StoreProp(id: Store<usize>) -> Element {
    rsx!(div { "{id}" })
}

#[component]
fn ReadStoreProp(id: ReadStore<usize>) -> Element {
    rsx!(div { "{id}" })
}

#[component]
fn WriteStoreProp(id: WriteStore<usize>) -> Element {
    rsx!(div { "{id}" })
}

#[component]
fn ReadSignalProp(id: ReadSignal<usize>) -> Element {
    rsx!(div { "{id}" })
}

#[component]
fn SyncReadSignalProp(id: ReadSignal<usize, SyncStorage>) -> Element {
    rsx!(div { "{id}" })
}

#[component]
fn WriteSignalProp(id: WriteSignal<usize>) -> Element {
    rsx!(div { "{id}" })
}

#[component]
fn ReadSignalStringProp(val: ReadSignal<String>) -> Element {
    rsx!(div { "{val}" })
}

#[component]
fn DefaultStoreProp(#[props(default = Store::new(5))] id: Store<usize>) -> Element {
    rsx!(div { "{id}" })
}

#[test]
fn store_props_do_not_leak() {
    // Mapped stores passed as Store/ReadStore/WriteStore props (issue #5671)
    assert_no_leak("Store<T> prop from mapped store", || {
        let data: Store<Vec<usize>> = use_store(|| (0..20).collect());
        rsx! {
            for id in data.iter() {
                StoreProp { id }
            }
        }
    });
    assert_no_leak("ReadStore<T> prop from mapped store", || {
        let data: Store<Vec<usize>> = use_store(|| (0..20).collect());
        rsx! {
            for id in data.iter() {
                ReadStoreProp { id }
            }
        }
    });
    assert_no_leak("WriteStore<T> prop from mapped store", || {
        let data: Store<Vec<usize>> = use_store(|| (0..20).collect());
        rsx! {
            for id in data.iter() {
                WriteStoreProp { id }
            }
        }
    });
    // Store::new allocates sync state for its subscription tree. If a store prop has a
    // default value, that state must be owned by the props, not leaked into the parent scope.
    assert_no_leak("Store<T> prop with default value", || {
        rsx! {
            for _ in 0..20 {
                DefaultStoreProp {}
            }
        }
    });
}

#[test]
fn signal_props_do_not_leak() {
    assert_no_leak("ReadSignal<T> prop from mapped signal", || {
        let data: Signal<Vec<usize>> = use_signal(|| (0..20).collect());
        rsx! {
            for i in 0..20 {
                ReadSignalProp { id: data.map(move |v| &v[i]) }
            }
        }
    });
    assert_no_leak("WriteSignal<T> prop from mapped signal", || {
        let data: Signal<Vec<usize>> = use_signal(|| (0..20).collect());
        rsx! {
            for i in 0..20 {
                WriteSignalProp { id: data.map_mut(move |v| &v[i], move |v| &mut v[i]) }
            }
        }
    });
    // Sync signals allocate with SyncStorage, which is owned separately from UnsyncStorage
    assert_no_leak(
        "ReadSignal<T, SyncStorage> prop from sync mapped signal",
        || {
            let data: Signal<Vec<usize>, SyncStorage> = use_signal_sync(|| (0..20).collect());
            rsx! {
                for i in 0..20 {
                    SyncReadSignalProp { id: data.map(move |v| &v[i]) }
                }
            }
        },
    );
    // Plain values converted into ReadSignal props create a fresh signal on every render.
    // The point_to memoization must not accumulate stale subscriptions in the child.
    assert_no_leak("ReadSignal<String> prop from plain value", || {
        rsx! {
            for i in 0..20 {
                ReadSignalStringProp { val: format!("value {i}") }
            }
        }
    });
}
