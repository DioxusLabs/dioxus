#![allow(non_snake_case)]
//! Micro-benchmarks for keyed fragment reconciliation.
//!
//! These deliberately avoid per-row components, signals-in-children, and dynamic
//! text so the only meaningful work per render is:
//!   1. running the app component (formatting `N` keys - common-mode cost), and
//!   2. the keyed children diff in `diff/iterator.rs`.
//!
//! Both costs are constant across runs, so the keyed diff itself shows up
//! cleanly here. `NoOpMutations` is used so we never measure the apply phase.
//!
//! The patterns cover the end-anchored cases (append, prepend, remove from an
//! end, single insert/remove, no-op re-render), the LIS-heavy cases (swap,
//! reverse, shuffle), and the disjoint-key path (replace all).

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use dioxus::prelude::*;
use dioxus_core::{NoOpMutations, ScopeId};
use rand::prelude::*;
use std::{cell::RefCell, hint::black_box, rc::Rc};

/// Number of rows in the steady-state list.
const N: u64 = 1_000;
/// How many rows the bulk insert/remove patterns touch.
const CHUNK: u64 = 100;
/// Key base for freshly created rows so they never collide with existing keys.
const FRESH: u64 = 1_000_000;

criterion_group!(benches, keyed_diff);
criterion_main!(benches);

fn keyed_diff(c: &mut Criterion) {
    let initial: Vec<u64> = (0..N).collect();
    let mut group = c.benchmark_group("keyed diff (1,000 rows)");

    // End-anchored cases (append, prepend, remove from an end, single edits).
    bench(&mut group, "append 100 at tail", &initial, append_tail);
    bench(&mut group, "prepend 100 at head", &initial, prepend_head);
    bench(&mut group, "remove 100 from tail", &initial, remove_tail);
    bench(&mut group, "remove 100 from head", &initial, remove_head);
    bench(&mut group, "insert 1 in middle", &initial, insert_middle);
    bench(&mut group, "remove 1 from middle", &initial, remove_middle);
    bench(
        &mut group,
        "re-render unchanged",
        &initial,
        rerender_unchanged,
    );

    // LIS-heavy / move-heavy cases.
    bench(&mut group, "swap ends", &initial, swap_ends);
    bench(&mut group, "reverse", &initial, reverse);
    bench(&mut group, "shuffle", &initial, shuffle);

    // Localized reorder inside a large stable list: the diff peels the shared
    // prefix/suffix and runs the LIS only over the 20-wide window, leaving the
    // ~490 stable rows on each side untouched.
    bench(&mut group, "reverse 20 mid-list", &initial, reverse_window);
    bench(&mut group, "shuffle 20 mid-list", &initial, shuffle_window);

    // No shared keys: the create-all + remove-all path.
    bench(&mut group, "replace all", &initial, replace_all);

    group.finish();
}

fn bench(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    initial: &[u64],
    mutate: fn(&mut Vec<u64>),
) {
    group.bench_function(name, |b| {
        b.iter_batched(
            || KeyedListDom::new(initial),
            |mut app| {
                app.mutate_and_render(mutate);
                black_box(app)
            },
            BatchSize::SmallInput,
        )
    });
}

// --- mutation patterns -------------------------------------------------------

fn append_tail(v: &mut Vec<u64>) {
    v.extend(FRESH..FRESH + CHUNK);
}

fn prepend_head(v: &mut Vec<u64>) {
    let mut new: Vec<u64> = (FRESH..FRESH + CHUNK).collect();
    new.append(v);
    *v = new;
}

fn remove_tail(v: &mut Vec<u64>) {
    v.truncate(v.len() - CHUNK as usize);
}

fn remove_head(v: &mut Vec<u64>) {
    v.drain(0..CHUNK as usize);
}

fn insert_middle(v: &mut Vec<u64>) {
    v.insert(v.len() / 2, FRESH);
}

fn remove_middle(v: &mut Vec<u64>) {
    v.remove(v.len() / 2);
}

fn rerender_unchanged(_v: &mut Vec<u64>) {}

fn swap_ends(v: &mut Vec<u64>) {
    let last = v.len() - 2;
    v.swap(1, last);
}

fn reverse(v: &mut Vec<u64>) {
    v.reverse();
}

fn shuffle(v: &mut Vec<u64>) {
    // Fixed seed: every batch reshuffles the same `initial` to the same target.
    let mut rng = SmallRng::seed_from_u64(0xD10C);
    v.shuffle(&mut rng);
}

fn replace_all(v: &mut Vec<u64>) {
    *v = (FRESH..FRESH + N).collect();
}

/// Reverse a 20-wide window in the middle, leaving ~490 stable rows on each side.
fn reverse_window(v: &mut Vec<u64>) {
    let mid = v.len() / 2;
    v[mid - 10..mid + 10].reverse();
}

/// Shuffle a 20-wide window in the middle, leaving ~490 stable rows on each side.
fn shuffle_window(v: &mut Vec<u64>) {
    let mid = v.len() / 2;
    let mut rng = SmallRng::seed_from_u64(0xD10C);
    v[mid - 10..mid + 10].shuffle(&mut rng);
}

// --- harness -----------------------------------------------------------------

#[derive(Clone)]
struct ListHandle(Rc<RefCell<Option<Signal<Vec<u64>>>>>);

impl PartialEq for ListHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// A minimal keyed list rendered straight to `NoOpMutations`.
fn keyed_list_app(handle: ListHandle) -> Element {
    let items = use_signal(Vec::<u64>::new);
    *handle.0.borrow_mut() = Some(items);
    rsx! {
        ul {
            for key in items.read().iter().copied() {
                li { key: "{key}" }
            }
        }
    }
}

struct KeyedListDom {
    dom: VirtualDom,
    items: Signal<Vec<u64>>,
}

impl KeyedListDom {
    /// Build the dom and bring it to the `initial` list as the steady state, so
    /// the measured render only performs the transition under test.
    fn new(initial: &[u64]) -> Self {
        let handle = ListHandle(Rc::new(RefCell::new(None)));
        let mut dom = VirtualDom::new_with_props(keyed_list_app, handle.clone());
        dom.rebuild(&mut NoOpMutations);

        let mut items = handle
            .0
            .borrow()
            .expect("signal is published during the first render");

        let initial = initial.to_vec();
        dom.runtime().in_scope(ScopeId::APP, || {
            *items.write() = initial;
        });
        dom.render_immediate(&mut NoOpMutations);

        Self { dom, items }
    }

    fn mutate_and_render(&mut self, mutate: fn(&mut Vec<u64>)) {
        let mut items = self.items;
        self.dom.runtime().in_scope(ScopeId::APP, || {
            let mut write = items.write();
            mutate(&mut write);
        });
        self.dom.render_immediate(&mut NoOpMutations);
    }
}
