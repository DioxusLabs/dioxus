use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dioxus::prelude::*;
use dioxus_tui::{Config, TuiContext};

criterion_group!(mbenches, tui_update);
criterion_main!(mbenches);

/// This benchmarks the cache performance of the TUI for small edits by changing one box at a time.
fn tui_update(c: &mut Criterion) {
    {
        let mut group = c.benchmark_group("Update boxes");

        for size in 1..=20usize {
            let parameter_string = format!("{}", (size).pow(2));
            group.bench_with_input(
                BenchmarkId::new("size", parameter_string),
                &size,
                |b, size| {
                    b.iter(|| {
                        dioxus_tui::launch_cfg_with_props(
                            app,
                            GridProps {
                                size: *size,
                                update_count: 1,
                            },
                            Config::default().with_headless(),
                        )
                    })
                },
            );
        }
    }

    {
        let mut group = c.benchmark_group("Update many boxes");

        for update_count in 1..=20usize {
            let update_count = update_count * 20;
            let parameter_string = update_count.to_string();
            group.bench_with_input(
                BenchmarkId::new("update count", parameter_string),
                &update_count,
                |b, update_count| {
                    b.iter(|| {
                        dioxus_tui::launch_cfg_with_props(
                            app,
                            GridProps {
                                size: 20,
                                update_count: *update_count,
                            },
                            Config::default().with_headless(),
                        )
                    })
                },
            );
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct BoxProps {
    x: usize,
    y: usize,
    hue: f32,
    alpha: f32,
}
#[allow(non_snake_case)]
fn Box(props: BoxProps) -> Element {
    let count = use_signal(|| 0);

    let x = props.x * 2;
    let y = props.y * 2;
    let hue = props.hue;
    let display_hue = props.hue as u32 / 10;
    let count = count();
    let alpha = props.alpha + (count % 100) as f32;

    rsx! {
        div {
            left: "{x}%",
            top: "{y}%",
            width: "100%",
            height: "100%",
            background_color: "hsl({hue}, 100%, 50%, {alpha}%)",
            align_items: "center",
            p{"{display_hue:03}"}
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct GridProps {
    size: usize,
    update_count: usize,
}
#[allow(non_snake_case)]
fn Grid(props: GridProps) -> Element {
    let size = props.size;
    let mut count = use_signal(|| 0);
    let mut counts = use_signal(|| vec![0; size * size]);

    let ctx: TuiContext = consume_context();
    if count() + props.update_count >= (size * size) {
        ctx.quit();
    } else {
        for _ in 0..props.update_count {
            counts.with_mut(|c| {
                let i = count();
                c[i] += 1;
                c[i] %= 360;
            });
            count.with_mut(|i| {
                *i += 1;
                *i %= size * size;
            });
        }
    }

    rsx! {
        div{
            width: "100%",
            height: "100%",
            flex_direction: "column",
            for x in 0..size {
                div {
                    width: "100%",
                    height: "100%",
                    flex_direction: "row",
                    for y in 0..size {
                        Box {
                            key: "{x}-{y}",
                            x: x,
                            y: y,
                            alpha: 100.0,
                            hue: y as f32*100.0/size as f32 + counts.read()[x*size + y] as f32,
                        }
                    }
                }
            }
        }
    }
}

fn app(props: GridProps) -> Element {
    rsx! {
        div{
            width: "100%",
            height: "100%",
            Grid{
                size: props.size,
                update_count: props.update_count,
            }
        }
    }
}
