use crate::{ClientStatus, DisconnectReason, LiveViewError, LiveViewSocket, WebSocketMsg as Msg};
use dioxus_core::prelude::*;
use futures_util::{pin_mut, SinkExt, StreamExt};
use std::ops::ControlFlow;
use std::rc::Rc;
use tokio::time::sleep;

const RENDER_DEADLINE: tokio::time::Duration = tokio::time::Duration::from_millis(10);

#[derive(Clone)]
pub struct LiveViewPool {
    pub(crate) pool: tokio_util::task::LocalPoolHandle,
}

impl Default for LiveViewPool {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveViewPool {
    pub fn new() -> Self {
        LiveViewPool {
            // TODO/XXX: This probably should default to the amount of
            // available cores, and be configurable. Relevant resources:
            // - https://doc.rust-lang.org/stable/std/thread/fn.available_parallelism.html
            // - https://docs.rs/num_cpus/latest/num_cpus/
            // - Discussions about `available_parallelism` vs. `num_cpus`
            //   - https://github.com/seanmonstar/num_cpus/issues/119
            //   - https://github.com/rayon-rs/rayon/pull/937
            //   - Looks like we probably should use `available_parallelism`
            pool: tokio_util::task::LocalPoolHandle::new(16),
        }
    }

    // TODO: Docs
    pub async fn launch<
        SendErr: Send + std::fmt::Debug + 'static,
        RecvErr: Send + std::fmt::Debug + 'static,
    >(
        &self,
        ws: impl LiveViewSocket<SendErr, RecvErr>,
        app: fn(Scope<()>) -> Element,
    ) -> DisconnectReason<SendErr, RecvErr> {
        self.launch_with_props(ws, app, ()).await
    }

    // TODO: Docs
    pub async fn launch_with_props<
        T: Send + 'static,
        SendErr: Send + std::fmt::Debug + 'static,
        RecvErr: Send + std::fmt::Debug + 'static,
    >(
        &self,
        ws: impl LiveViewSocket<SendErr, RecvErr>,
        app: fn(Scope<T>) -> Element,
        props: T,
    ) -> DisconnectReason<SendErr, RecvErr> {
        self.launch_with_props_and_client_status(ws, app, props, ClientStatus::Initiated)
            .await
    }

    // TODO: Docs
    // XXX: `ClientStatus` can be used to specify if the initial render is for
    // a client connecting or reconnecting, which is useful for restoring state
    // (e.g. when the client reconnects). After the first render, we set the
    // status to `ClientStatus::Initiated`.
    pub async fn launch_with_props_and_client_status<
        T: Send + 'static,
        SendErr: Send + std::fmt::Debug + 'static,
        RecvErr: Send + std::fmt::Debug + 'static,
    >(
        &self,
        ws: impl LiveViewSocket<SendErr, RecvErr>,
        app: fn(Scope<T>) -> Element,
        props: T,
        client_status: ClientStatus,
    ) -> DisconnectReason<SendErr, RecvErr> {
        let result = self
            .pool
            .spawn_pinned(move || run(app, props, ws, client_status))
            .await;

        match result {
            Ok(result) => result,
            Err(join_error) => {
                DisconnectReason::Error(LiveViewError::Panicked(join_error.into_panic()))
                // Note: `into_panic` panics if joining the spawned task failed
                // because the task was canceled. Because we don't cancel the
                // task, this can't happen.
            }
        }
    }
}

// XXX: I think, it would be better to use a builder instead of three `launch*`
// methods. However, I didn't manage to implement such a builder because of the
// requirement for generic props. Do you have an idea, how to implement this?
// Here is what I've tried:

// pub struct Builder<'pool, Props: Send + 'static = ()> {
//     pool: &'pool LiveViewPool,
//     props: Option<Props>,
//     client_status: ClientStatus,
// }

// impl<'pool, Props: Send + 'static> Builder<'pool, Props> {
//     pub fn with_props(self, props: Props) -> Self {
//         self.props = Some(props);
//         self
//     }

//     pub fn with_client_status(self, status: ClientStatus) -> Self {
//         self.client_status = status;
//         self
//     }

//     pub async fn lauch(
//         self,
//         app: fn(Scope<Props>) -> Element,
//         ws: impl LiveViewSocket,
//     ) -> Result<Option<CloseFrame>, LiveViewError> {
//         let p = self.pool.pool;
//         if let Some(props) = self.props {
//             let r = p
//                 .spawn_pinned(move || run(app, props, ws, self.client_status))
//                 .await;
//             match r {
//                 Ok(result) => result,
//                 Err(join_error) => Err(LiveViewError::Panicked(join_error.into_panic())),
//                 // Note: `into_panic` panics if joining the spawned task failed because the
//                 // task was canceled. Because we don't cancle the task, this can't happen.
//             }
//         } else {
//             let r = p
//                 .spawn_pinned(move || run(app, (), ws, self.client_status))
//                 .await;
//             match r {
//                 Ok(result) => result,
//                 Err(join_error) => Err(LiveViewError::Panicked(join_error.into_panic())),
//                 // Note: `into_panic` panics if joining the spawned task failed because the
//                 // task was canceled. Because we don't cancle the task, this can't happen.
//             }
//         }
//     }
// }

/// Desktop uses this wrapper struct thing around the actual event itself
/// this is sorta driven by tao/wry
#[derive(serde::Deserialize)]
struct IpcMessage {
    params: dioxus_html::HtmlEvent,
}

/// The primary event loop for the VirtualDom waiting for user input
///
/// This function makes it easy to integrate Dioxus LiveView with any socket-based framework.
///
/// As long as your framework can provide a Sink and Stream of Strings, you can use this function.
///
/// You might need to transform the error types of the web backend into the LiveView error type.
// TODO: Add docs
pub async fn run<
    T: Send + 'static,
    SendErr: Send + std::fmt::Debug + 'static,
    RecvErr: Send + std::fmt::Debug + 'static,
>(
    app: Component<T>,
    props: T,
    ws: impl LiveViewSocket<SendErr, RecvErr>,
    client_status: ClientStatus,
) -> DisconnectReason<SendErr, RecvErr> {
    #[cfg(all(feature = "hot-reload", debug_assertions))]
    let mut hot_reload_rx = {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        dioxus_hot_reload::connect(move |template| {
            let _ = tx.send(template);
        });
        rx
    };

    pin_mut!(ws);

    let (mut vdom, handlers) = match initiate_client(&mut ws, app, props, client_status).await {
        Err(err) => return DisconnectReason::Error(err),
        Ok(ok) => ok,
    };

    loop {
        #[cfg(all(feature = "hot-reload", debug_assertions))]
        let hot_reload_wait = hot_reload_rx.recv();
        #[cfg(not(all(feature = "hot-reload", debug_assertions)))]
        let hot_reload_wait = std::future::pending();

        tokio::select! {
            _ = vdom.wait_for_work() => (), // poll any futures or suspense

            msg = ws.next() => match handle_message(msg, &mut vdom, &mut ws, &handlers).await {
                ControlFlow::Break(result) => return result,
                ControlFlow::Continue(()) => (),
            },

            msg = hot_reload_wait => {
                if let Some(msg) = msg {
                    match msg{
                        dioxus_hot_reload::HotReloadMsg::UpdateTemplate(new_template) => {
                            vdom.replace_template(new_template);
                        }
                        dioxus_hot_reload::HotReloadMsg::Shutdown => {
                            std::process::exit(0);
                        },
                    }
                }
            }
        };

        let edits = vdom.render_with_deadline(sleep(RENDER_DEADLINE)).await;

        // TODO: Use a serialization error variant, or describe why this can't fail:
        let msg = Msg::Text(serde_json::to_string(&edits).unwrap());

        if let Err(e) = ws.send(msg).await {
            return DisconnectReason::Error(e);
        }

        // TODO: Implement streaming as described in
        // https://docs.rs/dioxus-core/latest/dioxus_core/prelude/struct.VirtualDom.html#use-with-streaming
        // XXX: Would that make sense?
    }
}

async fn initiate_client<
    T: Send + 'static,
    SendErr: Send + std::fmt::Debug + 'static,
    RecvErr: Send + std::fmt::Debug + 'static,
>(
    ws: &mut std::pin::Pin<&mut impl LiveViewSocket<SendErr, RecvErr>>,
    app: Component<T>,
    props: T,
    client_status: ClientStatus,
) -> Result<
    (VirtualDom, crate::DisconnectHandlers<SendErr, RecvErr>),
    LiveViewError<SendErr, RecvErr>,
> {
    let mut vdom = VirtualDom::new_with_props(app, props);

    // Provide a containers in which handlers and actions can be stored:

    let handlers: crate::hooks::DisconnectHandlers<_, _> =
        Rc::new(std::cell::RefCell::new(Vec::new()));

    let actions: crate::hooks::DisconnectClientActions =
        Rc::new(std::cell::RefCell::new(Vec::new()));

    vdom.base_scope().provide_context(Rc::clone(&handlers));
    vdom.base_scope().provide_context(Rc::clone(&actions));
    vdom.base_scope().provide_context(client_status); // Set `ClientState` during first render

    // Create the first message, which send the initial state of the app:

    let msg = {
        let msg = ClientInitiation {
            edits: vdom.rebuild(),
            // `vdom.rebuild()` also collects the `DisconnectClientActions`:
            on_disconnect: &actions.borrow(),
        };

        // TODO: use an efficient binary packed format for this. XXX: How
        // would it even be possible to use a binary format? Isn't JSON the
        // only format that's supported naively in the browser?
        // TODO: Use a deserialization error variant, or describe why this can't fail:
        serde_json::to_string(&msg).unwrap()
    };

    // Note: We always fully render the app, not matter if the client connects
    // for the first time, or reconnects. We do not try to store `VirtualDom`
    // instances of disconnected clients, and try to re-assign reconnecting
    // clients to their previous instance. To be able to reuse a `VirtualDom` in
    // a distributed setup (i.e. more than one server), `VirtualDom` would need
    // to be (de-)serializable, and we'd have to be able to send serialized
    // VDOMs to other servers, which would add  complexity, without significant
    // benefits (full renders anyway need to be fast, and state can be restored
    // with the tools we offer).

    // TODO: If the creation of `VirtualDom` is expensive, we may be able to
    // maintain a pool of instances that get reused.

    // XXX: Is creating VDOMs expensive? If so: Would using a VDOM pool  even
    // be possible? To prevent data leaks and security vulnerabilities it would
    // need to be guaranteed that all data store in a VDOM instance would be
    // discarded, before it's reused.

    // After the initial render, `ClientStatus` will always be `Initiated`:
    vdom.base_scope().provide_context(ClientStatus::Initiated);

    ws.send(Msg::Text(msg)).await?;

    Ok((vdom, handlers))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ClientInitiation<'a> {
    edits: dioxus_core::Mutations<'a>,
    on_disconnect: &'a Vec<crate::DisconnectClientAction>,
}

async fn handle_message<
    SendErr: Send + std::fmt::Debug + 'static,
    RecvErr: Send + std::fmt::Debug + 'static,
>(
    msg: Option<Result<Msg, LiveViewError<SendErr, RecvErr>>>,
    vdom: &mut VirtualDom,
    ws: &mut std::pin::Pin<&mut impl LiveViewSocket<SendErr, RecvErr>>,
    handlers: &crate::DisconnectHandlers<SendErr, RecvErr>,
) -> ControlFlow<DisconnectReason<SendErr, RecvErr>> {
    log::trace!("Received message: {msg:?}");

    let fail = |reason| handle_disconnect(DisconnectReason::Error(reason), handlers);

    let Some(Ok(Msg::Text(msg))) = msg else {
        // The client did disconnect, or an error happened and we should fail:
        return match msg {
            None => fail(LiveViewError::StreamClosedUnexpectedly {
                context: "While waiting for WebSocket message".into(),
            }),
            Some(Err(err)) => fail(err), // = `LiveViewError::ReceivingMsgFailed`
            Some(Ok(msg)) => match msg {
                Msg::Text(_) => unreachable!("Handled above, via `let/else`"),
                Msg::Binary(_) => fail(LiveViewError::UnexpectedMsg {
                    context: "Binary messages are currently unsupported".into(),
                    msg,
                }),
                Msg::Close(frame) => handle_disconnect(DisconnectReason::Closure(frame), handlers),
            },
        }
    };

    if msg == "__ping__" {
        // Keep WebSocket alive:
        if let Err(err) = ws.send(Msg::Text("__pong__".into())).await {
            return fail(err); // = `LiveViewError::SendingMsgFailed`
        }
        ControlFlow::Continue(())
    } else if let Ok(IpcMessage { params }) = serde_json::from_str(&msg) {
        vdom.handle_event(
            &params.name,
            params.data.into_any(),
            params.element,
            params.bubbles,
        );
        ControlFlow::Continue(())
    } else {
        fail(LiveViewError::UnexpectedMsg {
            msg: Msg::Text(msg),
            context: "Invalid message".into(),
        })
    }
}

/// Calls the disconnect handlers (if any are installed), and returns a
/// `ControlFlow` suitable for returning from `handle_message`
fn handle_disconnect<
    SendErr: Send + std::fmt::Debug + 'static,
    RecvErr: Send + std::fmt::Debug + 'static,
>(
    reason: DisconnectReason<SendErr, RecvErr>,
    handlers: &crate::DisconnectHandlers<SendErr, RecvErr>,
) -> ControlFlow<DisconnectReason<SendErr, RecvErr>> {
    log::debug!("A client is or will be disconnected"); // TODO: Remove

    // `handlers` isn't needed anymore, after this function call, so we can
    // consume it's content (this way the contained closures can be `FnOnce`):
    for handler in handlers.borrow_mut().drain(0..) {
        handler(&reason)
    }

    ControlFlow::Break(reason)
}

// TODO: Is there any reasonable way to call the disconnect handlers when the
// server crashes? Without this, all client session state will be lost. Right
// now, to make state crash resistant, it has to be saved on every change.

// I've found the `crash-handler` crate by Embark Studios, but they recommend to
// run as little code as possible in crash handlers:
// https://docs.rs/crash-handler/0.5.1/crash_handler/trait.CrashEvent.html#safety

// We possible could handle crashes within the session by using `catch_unwind`
// within the user session task, or from `launch_with_props_with_client_status`
// by handling the `JoinError`. The later would mean that we have to move the
// creation of the container with the event handlers from `initiate_client` to
// `launch_with_props`, and switch from `Rc` to `Arc`.

// TODO: At least, this should be documented

// XXX: Do you have any thoughts regarding the above?

// If `catch_unwind` is doable, then we could at least support that. I doubt
// that the code is `UnwindSafe`, but maybe we can just document, that if
// the reason for the client disconnecting is `DisconnectReason::Crash`, then
// whatever is done, needs to be done carefully (in a way that avoids data
// corruption).

// Regular shutdowns (e.g. shutdowns via signals like SIGTERM on Linux) could
// probably also be handled properly. However, this most-likely would also
// mean that the container containing the event handlers needs to be globally
// available (and protected via a Mutex or similar).

// Otherwise, it's probably reasonable to just document, that state is not
// crash resistant, and that important data should be saved on every change ,
// as I've done in `sessions.rs`.

// That would definitely be the simplest solution.
