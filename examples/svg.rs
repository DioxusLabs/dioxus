// Thanks to @japsu and their project https://github.com/japsu/jatsi for the example!

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let val = use_state(cx, || 5);

    cx.render(rsx! {
        div {
            user_select: "none",
            webkit_user_select: "none",
            margin_left: "10%",
            margin_right: "10%",
            h1 { "Click die to generate a new value" }
            div {
                cursor: "pointer",
                height: "80%",
                width: "80%",
                Die {
                    value: **val,
                    keep: true,
                    onclick: move |_| {
                        use rand::Rng;
                        let mut rng = rand::thread_rng();
                        val.set(rng.gen_range(1..=6));
                    }
                }
            }
        }
    })
}

#[derive(Props)]
pub struct DieProps<'a> {
    pub value: u64,
    pub keep: bool,
    pub onclick: EventHandler<'a, MouseEvent>,
}

const DOTS: [(i64, i64); 7] = [(-1, -1), (-1, -0), (-1, 1), (1, -1), (1, 0), (1, 1), (0, 0)];
const DOTS_FOR_VALUE: [[bool; 7]; 6] = [
    [false, false, false, false, false, false, true],
    [false, false, true, true, false, false, false],
    [false, false, true, true, false, false, true],
    [true, false, true, true, false, true, false],
    [true, false, true, true, false, true, true],
    [true, true, true, true, true, true, false],
];

const OFFSET: i64 = 600;
const DOT_RADIUS: &str = "200";
const HELD_COLOR: &str = "#aaa";
const UNHELD_COLOR: &str = "#ddd";

// A six-sided die (D6) with dots.
#[allow(non_snake_case)]
pub fn Die<'a>(cx: Scope<'a, DieProps<'a>>) -> Element {
    let &DieProps { value, keep, .. } = cx.props;

    let active_dots = &DOTS_FOR_VALUE[(value - 1) as usize];
    let fill = if keep { HELD_COLOR } else { UNHELD_COLOR };
    let dots = DOTS
        .iter()
        .zip(active_dots.iter())
        .filter(|(_, &active)| active)
        .map(|((x, y), _)| {
            let dcx = x * OFFSET;
            let dcy = y * OFFSET;

            rsx! {
                circle {
                    cx: "{dcx}",
                    cy: "{dcy}",
                    r: "{DOT_RADIUS}",
                    fill: "#333"
                }
            }
        });

    cx.render(rsx! {
      svg {
        onclick: move |e| cx.props.onclick.call(e),
        prevent_default: "onclick",
        class: "die",
        view_box: "-1000 -1000 2000 2000",

        rect {
          x: "-1000",
          y: "-1000",
          width: "2000",
          height: "2000",
          rx: "{DOT_RADIUS}",
          fill: "{fill}",
        }

        {dots}
      }
    })
}
