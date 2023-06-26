use criterion::{black_box, criterion_group, criterion_main, Criterion};

use dioxus::prelude::*;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| {
        b.iter(|| {
            //
            let mut dom = VirtualDom::new(app);
            dom.rebuild();
        })
    });
}

fn app(cx: Scope) -> Element {
    let val = use_state(cx, || 10);
    use_state(cx, || 10);
    use_state(cx, || 10);
    use_state(cx, || 10);
    use_state(cx, || 10);

    cx.render(rsx! {
        div {
            (0..100).map(|i| rsx! {
                div {
                    h1 { "hello world! "}
                    child_component { val: val.clone() }
                    p { "{i}" }
                }
            })
        }
    })
}

#[inline_props]
fn child_component(cx: Scope, val: UseState<i32>) -> Element {
    render! {
        "hello {val}"
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
