use dioxus::prelude::*;

fn main() {
    dioxus::tui::launch_cfg(
        app,
        dioxus::tui::Config {
            rendering_mode: dioxus::tui::RenderingMode::Rgb,
        },
    );
}

#[derive(Props, PartialEq)]
struct BoxProps {
    x: i32,
    y: i32,
    hue: f32,
    alpha: f32,
}
#[allow(non_snake_case)]
fn Box(cx: Scope<BoxProps>) -> Element {
    let count = use_state(&cx, || 0);

    use_future(&cx, (), move |_| {
        let count = count.to_owned();
        let update = cx.schedule_update();
        async move {
            loop {
                count.with_mut(|i| *i += 1);
                tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                update();
            }
        }
    });

    let x = cx.props.x * 2;
    let y = cx.props.y * 2;
    let hue = cx.props.hue;
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
            p{"{count}"}
        }
    })
}

fn app(cx: Scope) -> Element {
    let steps = 50;
    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            flex_direction: "column",
            (0..=steps).map(|x|
                {
                    let hue = x as f32*360.0/steps as f32;
                    cx.render(rsx! {
                        div{
                            width: "100%",
                            height: "100%",
                            flex_direction: "row",
                            (0..=steps).map(|y|
                                {
                                    let alpha = y as f32*100.0/steps as f32;
                                    cx.render(rsx! {
                                        Box{
                                            x: x,
                                            y: y,
                                            alpha: alpha,
                                            hue: hue,
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
