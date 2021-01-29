use dioxus::prelude::*;
use tide::{with_state, Request};
use tide_websockets::{WebSocket, WebSocketConnection};

fn main() {
    let mut g = Context { props: &() };
    LiveComponent(&mut g);
}

#[cfg(not(target_arch = "wasm32"))]
async fn server() -> tide::Result<()> {
    // Build the API
    let mut app = tide::with_state(());
    app.at("/app").get(WebSocket::new(live_handler));

    // Launch the server
    app.listen("127.0.0.1:8080").await?;

    Ok(())
}

async fn live_handler(req: Request<()>, stream: WebSocketConnection) -> tide::Result<()> {
    Ok(())
}

static LiveComponent: FC<()> = |ctx| {
    use_live_component(
        ctx,
        #[cfg(target_arch = "wasm32")]
        || {
            // Always wait on the context's live component API
            // suspend the component until this promise arrives, or fall back
            let g = &LiveComponent;
            html! {
                <div>
                   {"Hello world!"}
                </div>
            }
        },
        #[cfg(not(target_arch = "wasm32"))]
        || {
            // actually use the code originally specified in the component
            // this gives use the function pointer. We don't necessarily get to hash the same pointer between two binaries
            // Some key will need to be made, likely based on the function parameter
            let g = &LiveComponent;
            html! {
                <div>
                    {"Hello world!"}
                </div>
            }
        },
    )
};

/// This hooks connects with the LiveContext at the top of the app.
fn use_live_component<T>(ctx: &mut Context<T>, b: fn() -> VNode) -> VNode {
    todo!()
}

/// LiveContext is a special, limited form of the context api that disallows the "Context API"
/// Its purpose is to shield the original Context where calls to use_context will fail. Instead of panicing and confusing
/// users, we simply disallow usage of "use_context" and "childen". In effect, only serialiable props can be allowed.
///
/// In the future, we might try to lift these restrictions (esp children since they are virtual) but are limited via the web connection
struct LiveContext {}
