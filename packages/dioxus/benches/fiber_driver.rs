#![allow(non_snake_case)]

use criterion::{Criterion, criterion_group, criterion_main};
use dioxus::prelude::*;
use dioxus_core::{NoOpMutations, RuntimeGuard, UpdatePriority};
use futures_util::{FutureExt, pin_mut};
use std::cell::RefCell;
use std::task::{Context, Poll, Waker};

thread_local! {
    static ROUND: RefCell<Option<Signal<u32>>> = const { RefCell::new(None) };
}

criterion_group!(benches, fiber_driver_large_prop_wave);
criterion_main!(benches);

fn fiber_driver_large_prop_wave(c: &mut Criterion) {
    c.bench_function("fiber driver large prop wave", |b| {
        let mut dom = VirtualDom::new(app);
        dom.rebuild();
        let runtime = dom.runtime();

        b.iter(|| {
            bump_round(runtime.clone());
            drive_fibers(&mut dom);
        });
    });
}

fn bump_round(runtime: std::rc::Rc<dioxus_core::Runtime>) {
    ROUND.with_borrow(|slot| {
        let mut round = slot.expect("round signal should be registered");
        let _runtime = RuntimeGuard::new(runtime);
        dioxus_core::with_update_priority(UpdatePriority::Transition, || {
            round += 1;
        });
    });
}

fn drive_fibers(dom: &mut VirtualDom) {
    let mut mutations = NoOpMutations;
    let fut = dom.render_concurrent_into(&mut mutations);
    pin_mut!(fut);
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    while matches!(fut.poll_unpin(&mut cx), Poll::Pending) {
        // The core yield future wakes itself, so keep polling until the render pass drains.
    }
}

fn app() -> Element {
    let round = use_signal(|| 0);
    ROUND.with_borrow_mut(|slot| *slot = Some(round));

    rsx! {
        for id in 0..1_000_usize {
            Row { key: "{id}", id, round: round() }
        }
    }
}

#[component]
fn Row(id: usize, round: u32) -> Element {
    rsx! {
        div { "{id}:{round}" }
    }
}
