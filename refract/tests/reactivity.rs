use std::cell::Cell;
use std::rc::Rc;

use refract::{Lens, ResourceState, Ui, VecLens, dyn_text, el, lens, text};

mod async_util {
    use std::cell::Cell;
    use std::future::Future;
    use std::pin::Pin;
    use std::rc::Rc;
    use std::task::{Context, Poll};

    /// A future resolved by hand from the test body.
    pub struct ManualHandle<T> {
        slot: Rc<Cell<Option<T>>>,
    }

    impl<T> ManualHandle<T> {
        pub fn resolve(&self, value: T) {
            self.slot.set(Some(value));
        }
    }

    pub struct ManualFuture<T> {
        slot: Rc<Cell<Option<T>>>,
    }

    impl<T> Future for ManualFuture<T> {
        type Output = T;

        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<T> {
            match self.slot.take() {
                Some(v) => Poll::Ready(v),
                None => Poll::Pending,
            }
        }
    }

    pub fn manual<T>() -> (ManualHandle<T>, ManualFuture<T>) {
        let slot = Rc::new(Cell::new(None));
        (ManualHandle { slot: slot.clone() }, ManualFuture { slot })
    }
}

struct App {
    count: i32,
    label: String,
}

fn app_ui() -> Ui<App> {
    Ui::new(App {
        count: 0,
        label: "hi".to_string(),
    })
}

#[test]
fn effect_reruns_on_write() {
    let mut ui = app_ui();
    let count = lens!(App => 0: count);
    let runs = Rc::new(Cell::new(0));
    let runs2 = runs.clone();
    ui.effect(move |ctx| {
        let _ = *ctx.get(count);
        runs2.set(runs2.get() + 1);
    });
    assert_eq!(runs.get(), 1);
    ui.with(|ctx| *ctx.write(count) += 1);
    assert_eq!(runs.get(), 2);
}

#[test]
fn sibling_fields_are_isolated() {
    let mut ui = app_ui();
    let count = lens!(App => 0: count);
    let label = lens!(App => 1: label);
    let runs = Rc::new(Cell::new(0));
    let runs2 = runs.clone();
    ui.effect(move |ctx| {
        let _ = ctx.get(label).len();
        runs2.set(runs2.get() + 1);
    });
    assert_eq!(runs.get(), 1);
    // Writing a sibling field must not rerun the label effect.
    ui.with(|ctx| *ctx.write(count) += 1);
    assert_eq!(runs.get(), 1);
    ui.with(|ctx| ctx.write(label).push('!'));
    assert_eq!(runs.get(), 2);
}

#[test]
fn whole_state_write_wakes_field_readers() {
    let mut ui = app_ui();
    let root = refract::Root::<App>::new();
    let count = lens!(App => 0: count);
    let runs = Rc::new(Cell::new(0));
    let runs2 = runs.clone();
    ui.effect(move |ctx| {
        let _ = *ctx.get(count);
        runs2.set(runs2.get() + 1);
    });
    ui.with(|ctx| {
        *ctx.write(root) = App {
            count: 9,
            label: "new".to_string(),
        };
    });
    assert_eq!(runs.get(), 2);
}

#[test]
fn write_guard_used_read_only_does_not_notify() {
    let mut ui = app_ui();
    let count = lens!(App => 0: count);
    let runs = Rc::new(Cell::new(0));
    let runs2 = runs.clone();
    ui.effect(move |ctx| {
        let _ = *ctx.get(count);
        runs2.set(runs2.get() + 1);
    });
    ui.with(|ctx| {
        // Only Deref, never DerefMut: no notification.
        let guard = ctx.write(count);
        let _ = *guard;
    });
    assert_eq!(runs.get(), 1);
}

struct Todos {
    items: Vec<Todo>,
}

#[derive(PartialEq, Clone)]
struct Todo {
    title: String,
    done: bool,
}

#[test]
fn index_lens_granularity() {
    let mut ui = Ui::new(Todos {
        items: vec![
            Todo {
                title: "a".into(),
                done: false,
            },
            Todo {
                title: "b".into(),
                done: false,
            },
        ],
    });
    let items = lens!(Todos => 0: items);
    let first_done = items.at(0).field(1, |t: &Todo| &t.done, |t| &mut t.done);
    let second_done = items.at(1).field(1, |t: &Todo| &t.done, |t| &mut t.done);
    let runs = Rc::new(Cell::new(0));
    let runs2 = runs.clone();
    ui.effect(move |ctx| {
        let _ = *ctx.get(first_done);
        runs2.set(runs2.get() + 1);
    });
    assert_eq!(runs.get(), 1);
    // Writing item 1 must not wake a reader of item 0.
    ui.with(|ctx| *ctx.write(second_done) = true);
    assert_eq!(runs.get(), 1);
    // Writing item 0 wakes it.
    ui.with(|ctx| *ctx.write(first_done) = true);
    assert_eq!(runs.get(), 2);
    // Writing the whole list wakes it too.
    ui.with(|ctx| {
        ctx.write(items).push(Todo {
            title: "c".into(),
            done: false,
        })
    });
    assert_eq!(runs.get(), 3);
}

#[test]
fn memo_is_lazy_and_equality_gated() {
    let mut ui = app_ui();
    let count = lens!(App => 0: count);
    let computes = Rc::new(Cell::new(0));
    let computes2 = computes.clone();
    let parity = ui.memo(move |ctx| {
        computes2.set(computes2.get() + 1);
        *ctx.get(count) % 2
    });
    // Lazy: not computed until read.
    assert_eq!(computes.get(), 0);
    assert_eq!(*ui.read_memo(parity), 0);
    assert_eq!(computes.get(), 1);
    // 0 -> 2: parity unchanged; dependent effects must not rerun.
    let runs = Rc::new(Cell::new(0));
    let runs2 = runs.clone();
    ui.effect(move |ctx| {
        let _ = *ctx.read_memo(parity);
        runs2.set(runs2.get() + 1);
    });
    assert_eq!(runs.get(), 1);
    ui.with(|ctx| *ctx.write(count) += 2);
    assert_eq!(
        runs.get(),
        1,
        "effect must not rerun when memo value is unchanged"
    );
    ui.with(|ctx| *ctx.write(count) += 1);
    assert_eq!(runs.get(), 2);
}

#[test]
fn diamond_recomputes_once() {
    let mut ui = app_ui();
    let count = lens!(App => 0: count);
    let left = ui.memo(move |ctx| *ctx.get(count) + 1);
    let right = ui.memo(move |ctx| *ctx.get(count) * 2);
    let computes = Rc::new(Cell::new(0));
    let computes2 = computes.clone();
    let sum = ui.memo(move |ctx| {
        computes2.set(computes2.get() + 1);
        *ctx.read_memo(left) + *ctx.read_memo(right)
    });
    assert_eq!(*ui.read_memo(sum), 1);
    assert_eq!(computes.get(), 1);
    ui.with(|ctx| *ctx.write(count) = 3);
    assert_eq!(*ui.read_memo(sum), 10);
    assert_eq!(
        computes.get(),
        2,
        "diamond must recompute the join exactly once"
    );
}

#[test]
fn memo_chain_check_propagation() {
    let mut ui = app_ui();
    let count = lens!(App => 0: count);
    let parity = ui.memo(move |ctx| *ctx.get(count) % 2);
    let label = ui.memo(move |ctx| {
        if *ctx.read_memo(parity) == 0 {
            "even"
        } else {
            "odd"
        }
    });
    let computes = Rc::new(Cell::new(0));
    let computes2 = computes.clone();
    let shout = ui.memo(move |ctx| {
        computes2.set(computes2.get() + 1);
        ctx.read_memo(label).to_uppercase()
    });
    assert_eq!(*ui.read_memo(shout), "EVEN");
    assert_eq!(computes.get(), 1);
    // 0 -> 2: parity unchanged, so the whole chain must cut off without
    // recomputing `shout`.
    ui.with(|ctx| *ctx.write(count) = 2);
    assert_eq!(*ui.read_memo(shout), "EVEN");
    assert_eq!(computes.get(), 1);
    ui.with(|ctx| *ctx.write(count) = 3);
    assert_eq!(*ui.read_memo(shout), "ODD");
    assert_eq!(computes.get(), 2);
}

#[test]
#[should_panic(expected = "reactive cycle detected")]
fn cycle_detection() {
    let mut ui = app_ui();
    let count = lens!(App => 0: count);
    // A memo that reads itself.
    let cyclic: Rc<Cell<Option<refract::Memo<i32>>>> = Rc::new(Cell::new(None));
    let cyclic2 = cyclic.clone();
    let memo = ui.memo(move |ctx| {
        let me = cyclic2.get().unwrap();
        *ctx.get(count) + *ctx.read_memo(me)
    });
    cyclic.set(Some(memo));
    let _ = ui.read_memo(memo);
}

#[test]
fn peek_does_not_subscribe() {
    let mut ui = app_ui();
    let count = lens!(App => 0: count);
    let runs = Rc::new(Cell::new(0));
    let runs2 = runs.clone();
    ui.effect(move |ctx| {
        let _ = *ctx.peek(count);
        runs2.set(runs2.get() + 1);
    });
    ui.with(|ctx| *ctx.write(count) += 1);
    assert_eq!(runs.get(), 1);
}

#[test]
fn untracked_reads_do_not_subscribe() {
    let mut ui = app_ui();
    let count = lens!(App => 0: count);
    let runs = Rc::new(Cell::new(0));
    let runs2 = runs.clone();
    ui.effect(move |ctx| {
        ctx.untracked(|ctx| {
            let _ = *ctx.get(count);
        });
        runs2.set(runs2.get() + 1);
    });
    ui.with(|ctx| *ctx.write(count) += 1);
    assert_eq!(runs.get(), 1);
}

#[test]
fn nested_effects_torn_down_on_parent_rerun() {
    let mut ui = app_ui();
    let count = lens!(App => 0: count);
    let label = lens!(App => 1: label);
    let child_runs = Rc::new(Cell::new(0));
    let child_runs2 = child_runs.clone();
    ui.effect(move |ctx| {
        let _ = *ctx.get(count);
        let child_runs3 = child_runs2.clone();
        ctx.effect(move |ctx| {
            let _ = ctx.get(label).len();
            child_runs3.set(child_runs3.get() + 1);
        });
    });
    assert_eq!(child_runs.get(), 1);
    // Parent reruns: old child is dropped, a fresh child runs once.
    ui.with(|ctx| *ctx.write(count) += 1);
    assert_eq!(child_runs.get(), 2);
    // The label write must only rerun the single live child, not the stale one.
    ui.with(|ctx| ctx.write(label).push('!'));
    assert_eq!(child_runs.get(), 3);
}

#[test]
fn resource_lifecycle() {
    use async_util::{ManualHandle, manual};

    struct Q {
        query: i32,
    }
    let mut ui = Ui::new(Q { query: 1 });
    let query = lens!(Q => 0: query);
    type Handles = Rc<Cell<Vec<(i32, ManualHandle<String>)>>>;
    let handles: Handles = Rc::new(Cell::new(Vec::new()));
    let handles2 = handles.clone();
    // Cell take/inspect/put-back instead of RefCell borrows.
    fn with<R>(h: &Handles, f: impl FnOnce(&Vec<(i32, ManualHandle<String>)>) -> R) -> R {
        let v = h.take();
        let r = f(&v);
        h.set(v);
        r
    }

    let res = ui.resource(move |ctx| {
        // Tracked: reading `query` here makes it a dependency.
        let q = *ctx.get(query);
        let (handle, future) = manual::<String>();
        let mut v = handles2.take();
        v.push((q, handle));
        handles2.set(v);
        future
    });

    assert!(matches!(ui.read_resource(res), ResourceState::Pending));
    assert_eq!(with(&handles, |v| v.len()), 1);

    // Resolve the first run.
    with(&handles, |v| v[0].1.resolve("one".to_string()));
    ui.run_until_settled();
    assert_eq!(
        *ui.read_resource(res),
        ResourceState::Ready("one".to_string())
    );

    // Changing the dependency cancels and reloads, keeping stale data.
    ui.with(|ctx| *ctx.write(query) = 2);
    assert_eq!(with(&handles, |v| v.len()), 2);
    assert_eq!(
        *ui.read_resource(res),
        ResourceState::Reloading("one".to_string())
    );

    with(&handles, |v| v[1].1.resolve("two".to_string()));
    ui.run_until_settled();
    assert_eq!(
        *ui.read_resource(res),
        ResourceState::Ready("two".to_string())
    );
}

#[test]
fn resource_completion_wakes_effects() {
    use async_util::manual;

    struct Empty;
    let mut ui = Ui::new(Empty);
    let (handle, future) = manual::<i32>();
    let future = Rc::new(Cell::new(Some(future)));
    let res = ui.resource(move |_ctx| future.take().expect("resource restarted unexpectedly"));
    let seen = Rc::new(Cell::new(-1));
    let seen2 = seen.clone();
    ui.effect(move |ctx| {
        if let Some(v) = ctx.read_resource(res).value() {
            seen2.set(*v);
        }
    });
    assert_eq!(seen.get(), -1);
    handle.resolve(42);
    ui.run_until_settled();
    assert_eq!(seen.get(), 42);
}

#[test]
fn dom_updates_in_place() {
    let mut ui = app_ui();
    let count = lens!(App => 0: count);
    let root = ui.mount(
        el("div")
            .attr("class", "counter")
            .child(text("count: "))
            .child(dyn_text(move |ctx| ctx.get(count).to_string())),
    );
    assert_eq!(
        ui.render_to_string(root),
        "<div class=\"counter\">count: 0</div>"
    );
    ui.with(|ctx| *ctx.write(count) = 7);
    assert_eq!(
        ui.render_to_string(root),
        "<div class=\"counter\">count: 7</div>"
    );
}

#[test]
fn dyn_children_rebuild_and_teardown() {
    let mut ui = Ui::new(Todos {
        items: vec![Todo {
            title: "a".into(),
            done: false,
        }],
    });
    let items = lens!(Todos => 0: items);
    // Memoize the length so editing one item's title (which structurally
    // overlaps the whole list) cuts off instead of rebuilding the list.
    let len = ui.memo(move |ctx| ctx.get(items).len());
    let text_effect_runs = Rc::new(Cell::new(0));
    let text_effect_runs2 = text_effect_runs.clone();
    let root = ui.mount(el("ul").dyn_children(move |ctx| {
        let n = *ctx.read_memo(len);
        let runs = text_effect_runs2.clone();
        (0..n)
            .map(|i| {
                let title = items.at(i).field(0, |t: &Todo| &t.title, |t| &mut t.title);
                let runs = runs.clone();
                el("li")
                    .child(dyn_text(move |ctx| {
                        runs.set(runs.get() + 1);
                        ctx.get(title).clone()
                    }))
                    .into()
            })
            .collect()
    }));
    assert_eq!(ui.render_to_string(root), "<ul><li>a</li></ul>");
    assert_eq!(text_effect_runs.get(), 1);

    // Pushing an item rebuilds the list; old per-item effects are torn down.
    ui.with(|ctx| {
        ctx.write(items).push(Todo {
            title: "b".into(),
            done: false,
        })
    });
    assert_eq!(ui.render_to_string(root), "<ul><li>a</li><li>b</li></ul>");
    assert_eq!(text_effect_runs.get(), 3);

    // Editing one title touches only that item's effect.
    ui.with(|ctx| {
        let title = items.at(1).field(0, |t: &Todo| &t.title, |t| &mut t.title);
        *ctx.write(title) = "B".to_string();
    });
    assert_eq!(ui.render_to_string(root), "<ul><li>a</li><li>B</li></ul>");
    assert_eq!(text_effect_runs.get(), 4);
}

#[test]
fn multiple_read_guards_coexist() {
    let mut ui = app_ui();
    let count = lens!(App => 0: count);
    let label = lens!(App => 1: label);
    ui.with(|ctx| {
        // Two simultaneous zero-copy reads: fine, they are plain `&T`.
        let a = ctx.get(count);
        let b = ctx.get(label);
        assert_eq!(*a, 0);
        assert_eq!(&*b, "hi");
    });
}
