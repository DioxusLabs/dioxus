use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use refract::ReadGuard;
use refract::prelude::*;

#[derive(PartialEq, Debug)]
struct App {
    count: i32,
    name: String,
}

fn counter() -> (Rc<Cell<usize>>, impl Fn() -> usize) {
    let cell = Rc::new(Cell::new(0));
    let reader = {
        let cell = cell.clone();
        move || cell.get()
    };
    (cell, reader)
}

#[test]
fn effect_reruns_on_write() {
    let store = Store::new(App {
        count: 0,
        name: "a".into(),
    });
    let count = lens!(store => 0: count);
    let (runs, run_count) = counter();
    effect(move || {
        let _ = *count.read();
        runs.set(runs.get() + 1);
    });
    assert_eq!(run_count(), 1);
    *count.write() += 1;
    assert_eq!(run_count(), 2);
}

#[test]
fn sibling_fields_do_not_cross_talk() {
    let store = Store::new(App {
        count: 0,
        name: "a".into(),
    });
    let count = lens!(store => 0: count);
    let name = lens!(store => 1: name);
    let (runs, run_count) = counter();
    effect(move || {
        let _ = *count.read();
        runs.set(runs.get() + 1);
    });
    assert_eq!(run_count(), 1);
    // Writing a sibling field must not rerun the count effect.
    name.write().push('b');
    assert_eq!(run_count(), 1);
    // Writing the whole store must rerun it (prefix rule).
    store.set(App {
        count: 5,
        name: "c".into(),
    });
    assert_eq!(run_count(), 2);
}

#[test]
fn write_guard_without_deref_mut_does_not_notify() {
    let store = Store::new(App {
        count: 0,
        name: "a".into(),
    });
    let count = lens!(store => 0: count);
    let (runs, run_count) = counter();
    effect(move || {
        let _ = *count.read();
        runs.set(runs.get() + 1);
    });
    assert_eq!(run_count(), 1);
    {
        let guard = count.write();
        // Only reads through the guard: no notification on drop.
        assert_eq!(*guard, 0);
    }
    assert_eq!(run_count(), 1);
}

#[test]
fn index_lens_granularity() {
    let store = Store::new(vec![1, 2, 3]);
    let first = store.index::<i32>(0);
    let second = store.index::<i32>(1);
    let (runs, run_count) = counter();
    effect(move || {
        let _ = *first.read();
        runs.set(runs.get() + 1);
    });
    assert_eq!(run_count(), 1);
    *second.write() = 20;
    assert_eq!(run_count(), 1);
    *first.write() = 10;
    assert_eq!(run_count(), 2);
    // Writing the whole vec wakes per-item subscribers.
    store.with_mut(|v| v.push(4));
    assert_eq!(run_count(), 3);
}

#[test]
fn memo_equality_cutoff() {
    let store = Store::new(App {
        count: 1,
        name: "a".into(),
    });
    let count = lens!(store => 0: count);
    let parity = memo(move || *count.read() % 2);
    let (runs, run_count) = counter();
    effect(move || {
        let _ = *parity.read();
        runs.set(runs.get() + 1);
    });
    assert_eq!(run_count(), 1);
    // 1 -> 3: parity unchanged, effect must not rerun.
    *count.write() = 3;
    assert_eq!(run_count(), 1);
    // 3 -> 4: parity changed.
    *count.write() = 4;
    assert_eq!(run_count(), 2);
}

#[test]
fn diamond_runs_once() {
    let store = Store::new(1i32);
    let double = memo(move || *store.read() * 2);
    let triple = memo(move || *store.read() * 3);
    let (runs, run_count) = counter();
    let sum = memo(move || *double.read() + *triple.read());
    effect(move || {
        let _ = *sum.read();
        runs.set(runs.get() + 1);
    });
    assert_eq!(run_count(), 1);
    store.set(2);
    assert_eq!(run_count(), 2);
    assert_eq!(*sum.read(), 10);
}

#[test]
fn lens_into_memo_is_zero_copy() {
    let store = Store::new(3i32);
    let pair = memo(move || {
        let v = *store.read();
        (v, v * v)
    });
    let square = pair.select(1, |p| &p.1, |p| &mut p.1);
    assert_eq!(*square.read(), 9);
    store.set(4);
    assert_eq!(*square.read(), 16);
    // Guard projection combinators also work.
    let guard: ReadGuard<i32> = ReadGuard::map(pair.read(), |p| &p.0);
    assert_eq!(*guard, 4);
}

#[test]
fn effect_rerun_drops_owned_children() {
    let outer = Store::new(0i32);
    let (runs, run_count) = counter();
    effect(move || {
        let generation = *outer.read();
        // This inner store and effect are owned by the outer effect and must
        // be torn down on every rerun.
        let inner = Store::new(generation);
        let runs = runs.clone();
        effect(move || {
            let _ = *inner.read();
            runs.set(runs.get() + 1);
        });
    });
    assert_eq!(run_count(), 1);
    outer.set(1);
    assert_eq!(run_count(), 2);
    outer.set(2);
    // If old inner effects leaked, the count would jump by more than one.
    assert_eq!(run_count(), 3);
}

#[test]
fn guard_keeps_value_alive_after_scope_drop() {
    let outer = Store::new(0i32);
    let stash: Rc<Cell<Option<Store<String>>>> = Rc::new(Cell::new(None));
    let stash2 = stash.clone();
    effect(move || {
        let _ = *outer.read();
        stash2.set(Some(Store::new("quarantined".to_string())));
    });
    let inner = stash.get().unwrap();
    let guard = inner.read();
    // Rerunning the outer effect drops the inner store while `guard` is
    // alive: the value must be quarantined, not freed.
    outer.set(1);
    assert!(!inner.is_alive());
    assert_eq!(&*guard, "quarantined");
    drop(guard);
}

#[test]
#[should_panic(expected = "reactive cycle")]
fn cycle_panics() {
    let store = Store::new(1i32);
    let stash: Rc<Cell<Option<Memo<i32>>>> = Rc::new(Cell::new(None));
    let stash2 = stash.clone();
    let cyclic = memo(move || {
        let inner = stash2.get();
        match inner {
            Some(memo) => *memo.read() + *store.read(),
            None => *store.read(),
        }
    });
    stash.set(Some(cyclic));
    let _ = *cyclic.read(); // first read: no cycle yet (stash was empty)
    store.set(2);
    let _ = *cyclic.read(); // now the memo reads itself
}

#[test]
fn untracked_reads_do_not_subscribe() {
    let store = Store::new(0i32);
    let (runs, run_count) = counter();
    effect(move || {
        let _ = untracked(|| *store.read());
        runs.set(runs.get() + 1);
    });
    assert_eq!(run_count(), 1);
    store.set(1);
    assert_eq!(run_count(), 1);
}

#[test]
fn peek_does_not_subscribe() {
    let store = Store::new(0i32);
    let (runs, run_count) = counter();
    effect(move || {
        let _ = *store.peek();
        runs.set(runs.get() + 1);
    });
    assert_eq!(run_count(), 1);
    store.set(1);
    assert_eq!(run_count(), 1);
}

mod async_util {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};

    /// A future resolved manually from the test.
    pub struct ManualFuture<T> {
        state: Arc<Mutex<(Option<T>, Option<Waker>)>>,
    }

    #[derive(Clone)]
    pub struct ManualHandle<T> {
        state: Arc<Mutex<(Option<T>, Option<Waker>)>>,
    }

    pub fn manual<T>() -> (ManualHandle<T>, ManualFuture<T>) {
        let state = Arc::new(Mutex::new((None, None)));
        (
            ManualHandle {
                state: state.clone(),
            },
            ManualFuture { state },
        )
    }

    impl<T> ManualHandle<T> {
        pub fn resolve(&self, value: T) {
            let mut state = self.state.lock().unwrap();
            state.0 = Some(value);
            if let Some(waker) = state.1.take() {
                waker.wake();
            }
        }
    }

    impl<T> Future for ManualFuture<T> {
        type Output = T;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
            let mut state = self.state.lock().unwrap();
            match state.0.take() {
                Some(value) => Poll::Ready(value),
                None => {
                    state.1 = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        }
    }
}

#[test]
fn resource_lifecycle() {
    use async_util::{ManualHandle, manual};
    use std::cell::RefCell;

    let query = Store::new(1i32);
    type Handles = Rc<RefCell<Vec<(i32, ManualHandle<String>)>>>;
    let handles: Handles = Rc::new(RefCell::new(Vec::new()));
    let handles2 = handles.clone();

    let res: Resource<String> = resource(move || {
        // Tracked: reading `query` here makes it a dependency.
        let q = *query.read();
        let (handle, future) = manual::<String>();
        handles2.borrow_mut().push((q, handle));
        future
    });

    assert!(matches!(*res.read(), ResourceState::Pending));
    assert!(res.try_read().is_none());

    handles.borrow()[0].1.resolve("one".to_string());
    assert!(run_until_settled(Duration::from_secs(1)));
    assert_eq!(res.try_read().as_deref(), Some(&"one".to_string()));

    // Changing the dependency restarts the future; the old value survives as
    // Reloading (moved, not cloned).
    query.set(2);
    assert!(matches!(
        &*res.read(),
        ResourceState::Reloading(v) if v == "one"
    ));
    assert_eq!(res.try_read().as_deref(), Some(&"one".to_string()));
    assert!(res.read().is_loading());

    assert_eq!(handles.borrow().len(), 2);
    assert_eq!(handles.borrow()[1].0, 2);
    handles.borrow()[1].1.resolve("two".to_string());
    assert!(run_until_settled(Duration::from_secs(1)));
    assert!(matches!(&*res.read(), ResourceState::Ready(v) if v == "two"));

    // Effects over resources rerun on completion.
    let (runs, run_count) = counter();
    effect(move || {
        let _ = res.read().value().cloned();
        runs.set(runs.get() + 1);
    });
    assert_eq!(run_count(), 1);
    query.set(3);
    assert_eq!(run_count(), 2); // Ready -> Reloading
    handles.borrow()[2].1.resolve("three".to_string());
    assert!(run_until_settled(Duration::from_secs(1)));
    assert_eq!(run_count(), 3); // Reloading -> Ready
}

#[test]
fn dom_updates_in_place() {
    let store = Store::new(App {
        count: 0,
        name: "world".into(),
    });
    let count = lens!(store => 0: count);
    let name = lens!(store => 1: name);

    let view = el("div")
        .attr("id", "root")
        .child(dyn_text(move || format!("hello {}", name.read())))
        .child(el("span").child(dyn_text(move || format!("count: {}", count.read()))));

    assert_eq!(
        view.render_to_string(),
        "<div id=\"root\">hello world<span>count: 0</span></div>"
    );
    *count.write() += 1;
    assert_eq!(
        view.render_to_string(),
        "<div id=\"root\">hello world<span>count: 1</span></div>"
    );
}

#[test]
fn dyn_children_rebuild_and_teardown() {
    let items = Store::new(vec!["a".to_string(), "b".to_string()]);
    let (runs, run_count) = counter();

    // Reading the whole vec in the list effect would subscribe it at the root
    // path, so any item write would rebuild the list. Subscribing through an
    // equality-gated `len` memo keeps the rebuild granularity at "list shape
    // changed" while per-item effects track individual items.
    let len = memo(move || items.read().len());
    let view = el("ul").dyn_children(move || {
        let runs = runs.clone();
        (0..*len.read())
            .map(|i| {
                let item = items.index::<String>(i);
                let runs = runs.clone();
                el("li").child(dyn_text(move || {
                    runs.set(runs.get() + 1);
                    item.read().clone()
                }))
            })
            .collect()
    });

    assert_eq!(view.render_to_string(), "<ul><li>a</li><li>b</li></ul>");
    assert_eq!(run_count(), 2);

    // Editing one item reruns only that item's text effect.
    *items.index::<String>(0).write() = "A".to_string();
    assert_eq!(view.render_to_string(), "<ul><li>A</li><li>b</li></ul>");
    assert_eq!(run_count(), 3);

    // Pushing rebuilds the list; old per-item effects are torn down, so the
    // count grows by exactly the new list length.
    items.with_mut(|v| v.push("c".to_string()));
    assert_eq!(
        view.render_to_string(),
        "<ul><li>A</li><li>b</li><li>c</li></ul>"
    );
    assert_eq!(run_count(), 6);
    // ...and editing an item after the rebuild still reruns exactly one.
    *items.index::<String>(2).write() = "C".to_string();
    assert_eq!(run_count(), 7);
}
