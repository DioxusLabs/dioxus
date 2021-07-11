use dioxus::prelude::*;

fn main() {}

static App: FC<()> = |cx| {
    //
    cx.render(rsx!(
        div {
            h1 {}
        }
    ))
};

#[test]
fn blah() {}
