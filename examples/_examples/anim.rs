fn main() {
    // render the
    let transition = move |cx, (width, height)| {};

    cx.render(rsx! {
        div {
            Transition {
                start: (0, 5),
                stop: (10, 10),
                render: transition
            }

            Transition {
                start: (0, 5),
                stop: (10, 10),
                render: move |cx, (width, height)| {
                    //
                    cx.render(rsx!{
                        div {
                            style {
                                width: width,
                                width: height
                            }
                        }
                    })
                }
            }
        }
    })
}

// Animations with signals
fn signal_based(cx: ()) {
    const InitPos: (i32, i32) = (0, 0);
    const EndPos: (i32, i32) = (100, 200);

    let spring = use_spring(cx, move |spring| spring.from(InitPos).to(EndPos));

    cx.render(rsx! {
        div {
            style: [
                width: spring.0,
                height: spring.1
            ]
            button { onclick: move |_| spring.set(InitPos), "Reset" }
            button { onclick: move |_| spring.set(EndPos), "Animate" }
        }
    })
}
