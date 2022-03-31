use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dioxus::prelude::*;
use dioxus_tui::TuiContext;

criterion_group!(mbenches, update_boxes);
criterion_main!(mbenches);

/// This benchmarks the cache performance of the TUI for small edits by changing one box at a time.
fn update_boxes(c: &mut Criterion) {
    let mut group = c.benchmark_group("Update boxes");
    // We can override the configuration on a per-group level
    group.sample_size(10);

    // We can also use loops to define multiple benchmarks, even over multiple dimensions.
    for size in 1..=6 {
        let parameter_string = format!("{}", 5 * size);
        group.bench_with_input(
            BenchmarkId::new("size", parameter_string),
            &size,
            |b, size| {
                b.iter(|| match size {
                    1 => dioxus::tui::launch(app5),
                    2 => dioxus::tui::launch(app10),
                    3 => dioxus::tui::launch(app15),
                    4 => dioxus::tui::launch(app20),
                    5 => dioxus::tui::launch(app25),
                    6 => dioxus::tui::launch(app30),
                    _ => (),
                })
            },
        );
    }
}

#[derive(Props, PartialEq)]
struct BoxProps {
    x: usize,
    y: usize,
    hue: f32,
    alpha: f32,
}
#[allow(non_snake_case)]
fn Box(cx: Scope<BoxProps>) -> Element {
    let count = use_state(&cx, || 0);

    let x = cx.props.x * 2;
    let y = cx.props.y * 2;
    let hue = cx.props.hue;
    let display_hue = cx.props.hue as u32 / 10;
    let count = count.get();
    let alpha = cx.props.alpha + (count % 100) as f32;

    cx.render(rsx! {
        div {
            left: "{x}%",
            top: "{y}%",
            width: "100%",
            height: "100%",
            background_color: "hsl({hue}, 100%, 50%, {alpha}%)",
            align_items: "center",
            p{"{display_hue:03}"}
        }
    })
}

#[derive(Props, PartialEq)]
struct GridProps {
    size: usize,
}
#[allow(non_snake_case)]
fn Grid(cx: Scope<GridProps>) -> Element {
    let size = cx.props.size;
    let count = use_state(&cx, || 0);
    let counts = use_ref(&cx, || vec![0; size * size]);

    let ctx: TuiContext = cx.consume_context().unwrap();
    if *count.get() + 1 >= (size * size) {
        ctx.quit();
    } else {
        counts.with_mut(|c| {
            let i = *count.current();
            c[i] += 1;
            c[i] = c[i] % 360;
        });
        count.with_mut(|i| {
            *i += 1;
            *i = *i % (size * size);
        });
    }

    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            flex_direction: "column",
            (0..size).map(|x|
                    {
                    cx.render(rsx! {
                        div{
                            width: "100%",
                            height: "100%",
                            flex_direction: "row",
                            (0..size).map(|y|
                                {
                                    let alpha = y as f32*100.0/size as f32 + counts.read()[x*size + y] as f32;
                                    let key = format!("{}-{}", x, y);
                                    cx.render(rsx! {
                                        Box{
                                            x: x,
                                            y: y,
                                            alpha: 100.0,
                                            hue: alpha,
                                            key: "{key}",
                                        }
                                    })
                                }
                            )
                        }
                    })
                }
            )
        }
    })
}

fn app5(cx: Scope) -> Element {
    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            Grid{
                size: 5,
            }
        }
    })
}

fn app10(cx: Scope) -> Element {
    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            Grid{
                size: 10,
            }
        }
    })
}

fn app15(cx: Scope) -> Element {
    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            Grid{
                size: 15,
            }
        }
    })
}

fn app20(cx: Scope) -> Element {
    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            Grid{
                size: 20,
            }
        }
    })
}

fn app25(cx: Scope) -> Element {
    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            Grid{
                size: 25,
            }
        }
    })
}

fn app30(cx: Scope) -> Element {
    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            Grid{
                size: 30,
            }
        }
    })
}
