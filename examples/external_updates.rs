//! Example: External Updates
//! -------------------------
//!
//! Cause updates to the VirtualDOM state from outside the component lifecycle.
//! The root props could be changed or the use_receiver hook could be used.
//!
//!

fn main() {
    let (recv, sender) = channel();

    async_std::task::spawn({
        for location in ["a", "b", "c", "d"] {
            sender.send(location);
        }
    });

    let app = diouxs_webview::launch_with_props(App, RootProps { recv }).await;
}

struct RootProps {
    navigator: Receiver<&'static str>,
}

fn App(ctx: Context, props: &RootProps) -> VNode {
    let router = use_router(&ctx, |router| {});

    let navigator = use_history(&ctx);

    use_receiver(&ctx, || ctx.recv.clone(), |to| navigator.navigate(to));

    ctx.render(rsx! {
        div {
            a { href: "/dogs/"}
            a { href: "/cats/"}
            {content}
        }
    })
}
