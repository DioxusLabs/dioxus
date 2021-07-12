use std::cell::Cell;

use dioxus::prelude::*;
use dioxus_core::{
    nodes::{NodeKey, VElement, VText},
    RealDomNode,
};

fn main() {
    env_logger::init();
    dioxus::desktop::launch(Example, |c| c);
}

const STYLE: &str = r#"
body {background-color: powderblue;}
h1   {color: blue;}
p    {color: red;}
"#;

const Example: FC<()> = |cx| {
    cx.render(rsx! {
        Fragment {
            Fragment {
                Fragment {
                    "h1"
                }
                "h2"
            }
            "h3"
        }
        "h4"
        div { "h5" }
        Child {}
    })
};

const Child: FC<()> = |cx| {
    cx.render(rsx!(
        h1 {"1" }
        h1 {"2" }
        h1 {"3" }
        h1 {"4" }
    ))
};

// this is a bad case that hurts our subtree memoization :(
const AbTest: FC<()> = |cx| {
    if 1 == 2 {
        cx.render(rsx!(
            h1 {"1"}
            h1 {"2"}
            h1 {"3"}
            h1 {"4"}
        ))
    } else {
        cx.render(rsx!(
            h1 {"1"}
            h1 {"2"}
            h2 {"basd"}
            h1 {"4"}
        ))
    }
};
