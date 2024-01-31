use dioxus::prelude::*;
use dioxus_tui::{Config, TuiContext};

/// This benchmarks the cache performance of the TUI for small edits by changing one box at a time.
fn main() {
    for size in 1..=20usize {
        for _ in 0..10 {
            let dom = VirtualDom::new(app).with_root_context(size);
            dioxus_tui::launch_vdom_cfg(dom, Config::default().with_headless())
        }
    }
}

fn app() -> Element {
    let size = use_context::<usize>();
    rsx! {
        div { width: "100%", height: "100%", Grid { size } }
    }
}

#[component]
fn Box(x: usize, y: usize, hue: f32, alpha: f32) -> Element {
    let count = use_signal(|| 0);

    let x = x * 2;
    let y = y * 2;
    let hue = hue;
    let display_hue = hue as u32 / 10;

    let alpha = alpha + (count() % 100) as f32;

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

#[component]
fn Grid(size: usize) -> Element {
    let size = size;
    let mut count = use_signal(|| 0);
    let mut counts = use_signal(|| vec![0; size * size]);

    let ctx: TuiContext = consume_context();

    if count() + 1 >= (size * size) {
        ctx.quit();
    } else {
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

    rsx! {
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
