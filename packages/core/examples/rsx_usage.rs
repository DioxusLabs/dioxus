use dioxus_core::prelude::*;

fn main() {}

trait SProps {}

trait Comp {
    type Props;
}

impl<T> Comp for FC<T> {
    type Props = T;
}

fn test() {
    // let g: FC<ButtonProps> = CustomButton;
}

trait Render: Sized {
    fn render(ctx: Context<Self>) -> VNode;
}
// include as much as you might accept
struct Button {
    onhover: Option<Box<dyn Fn()>>,
}

impl Render for Button {
    fn render(ctx: Context<Self>) -> VNode {
        let _onfocus = move |_evt: ()| log::debug!("Focused");

        // todo!()
        ctx.render(rsx! {
            button {
                // ..ctx.attrs,
                class: "abc123",
                // style: { a: 2, b: 3, c: 4 },
                onclick: move |_evt| {
                    // log::info("hello world");
                },
                // Custom1 { a: 123 }
                // Custom2 { a: 456, "abc", h1 {"1"}, h2 {"2"} }
                // Custom3 { a: "sometext goes here" }
                // Custom4 { onclick: |evt| log::info("click") }
            }
        })
    }
}
