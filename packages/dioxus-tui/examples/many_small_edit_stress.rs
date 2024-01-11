use dioxus::prelude::*;
use dioxus_tui::{Config, TuiContext};

/// This benchmarks the cache performance of the TUI for small edits by changing one box at a time.
fn main() {
    for size in 1..=20usize {
        for _ in 0..10 {
            dioxus_tui::launch_cfg_with_props(app, size, Config::default().with_headless())
        }
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
    let count = use_state(cx, || 0);

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
    let count = use_state(cx, || 0);
    let counts = use_ref(cx, || vec![0; size * size]);

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

    render! {
        div{
            width: "100%",
            height: "100%",
            flex_direction: "column",
            for x in 0..size {
                div{
                    width: "100%",
                    height: "100%",
                    flex_direction: "row",
                    for y in 0..size {
                        {
                            let alpha = y as f32*100.0/size as f32 + counts.read()[x*size + y] as f32;
                            let key = format!("{}-{}", x, y);
                            rsx! {
                                Box {
                                    x: x,
                                    y: y,
                                    alpha: 100.0,
                                    hue: alpha,
                                    key: "{key}",
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn app(cx: Scope<usize>) -> Element {
    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            Grid{
                size: *cx.props,
            }
        }
    })
}
