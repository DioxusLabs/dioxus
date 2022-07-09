use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dioxus::prelude::*;
use dioxus_tui::{Config, TuiContext};

criterion_group!(mbenches, tui_update);
criterion_main!(mbenches);

/// This benchmarks the cache performance of the TUI for small edits by changing one box at a time.
fn tui_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("Update boxes");

    // We can also use loops to define multiple benchmarks, even over multiple dimensions.
    for size in 1..=8u32 {
        let parameter_string = format!("{}", (3 * size).pow(2));
        group.bench_with_input(
            BenchmarkId::new("size", parameter_string),
            &size,
            |b, size| {
                b.iter(|| match size {
                    1 => dioxus_tui::launch_cfg(app3, Config::default().with_headless()),
                    2 => dioxus_tui::launch_cfg(app6, Config::default().with_headless()),
                    3 => dioxus_tui::launch_cfg(app9, Config::default().with_headless()),
                    4 => dioxus_tui::launch_cfg(app12, Config::default().with_headless()),
                    5 => dioxus_tui::launch_cfg(app15, Config::default().with_headless()),
                    6 => dioxus_tui::launch_cfg(app18, Config::default().with_headless()),
                    7 => dioxus_tui::launch_cfg(app21, Config::default().with_headless()),
                    8 => dioxus_tui::launch_cfg(app24, Config::default().with_headless()),
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
            c[i] %= 360;
        });
        count.with_mut(|i| {
            *i += 1;
            *i %= size * size;
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

fn app3(cx: Scope) -> Element {
    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            Grid{
                size: 3,
            }
        }
    })
}

fn app6(cx: Scope) -> Element {
    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            Grid{
                size: 6,
            }
        }
    })
}

fn app9(cx: Scope) -> Element {
    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            Grid{
                size: 9,
            }
        }
    })
}

fn app12(cx: Scope) -> Element {
    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            Grid{
                size: 12,
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

fn app18(cx: Scope) -> Element {
    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            Grid{
                size: 18,
            }
        }
    })
}

fn app21(cx: Scope) -> Element {
    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            Grid{
                size: 21,
            }
        }
    })
}

fn app24(cx: Scope) -> Element {
    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            Grid{
                size: 24,
            }
        }
    })
}
