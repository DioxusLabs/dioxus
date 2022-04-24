use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            button {
                ondblclick: move |_| {
                    //
                    println!("double clicked!");
                },
                "Click me!"
            }
            input {
                 onfocusin: move |_| {
                    //
                    println!("blurred!");
                },
                "onblur": "console.log('blurred!')"
            }
        }
    })
}
