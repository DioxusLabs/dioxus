use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
}

criterion_group!(benches, criterion_benchmark);

fn main() {
    benches();
    Criterion::default().configure_from_args().final_summary();
    // $crate::__warn_about_html_reports_feature();
    // $crate::__warn_about_cargo_bench_support_feature();
}
