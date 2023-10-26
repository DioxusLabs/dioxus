#![allow(unused)]
use generational_box::{GenerationalBox, Owner, Store};

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn create(owner: &Owner) -> GenerationalBox<u32> {
    owner.insert(0)
}

fn set_read(signal: GenerationalBox<u32>) -> u32 {
    signal.set(1);
    *signal.read()
}

fn bench_fib(c: &mut Criterion) {
    let store = Store::default();
    let owner = store.owner();
    c.bench_function("create", |b| b.iter(|| create(black_box(&owner))));
    let signal = create(&owner);
    c.bench_function("set_read", |b| b.iter(|| set_read(black_box(signal))));
}

criterion_group!(benches, bench_fib);
criterion_main!(benches);
