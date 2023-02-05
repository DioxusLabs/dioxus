use std::{borrow::Cow, cell::RefCell, rc::Rc};

use dioxus_core::ScopeState;

pub(crate) type DisconnectHandlers<SendErr, RecvErr> =
    Rc<RefCell<Vec<Box<dyn FnOnce(&crate::DisconnectReason<SendErr, RecvErr>) + 'static>>>>;

// TODO: Docs
pub fn use_disconnect_handler<
    SendErr: Send + std::fmt::Debug + 'static,
    RecvErr: Send + std::fmt::Debug + 'static,
>(
    cx: &ScopeState,
    handler: impl FnOnce(&crate::DisconnectReason<SendErr, RecvErr>) + 'static,
) {
    cx.use_hook(move || {
        // XXX: This only uses a hook to make sure that `handler` is only added
        // once. If there was a way to do that in a different way, we could
        // name this function `on_disconnect` (i.e. not treat it as a hook),
        // which seems nicer.
        log::debug!("Storing disconnect handler"); // TODO: Remove when example if finished

        // XXX: A way to access the base scope would be useful.
        // `DisconnectHandlers` is always in `ScopeId(0)`:
        let handlers = cx
            .consume_context::<DisconnectHandlers<SendErr, RecvErr>>()
            .expect("`DisconnectHandlers` should be provided by `pool::run`");

        let mut hh = handlers.borrow_mut();
        hh.push(Box::new(handler));
        // XXX: There isn't a reasonable way to avoid boxing the closure, right?
    });
}

// TODO/XXX: Should disconnect handlers even get access to the reason of the
// disconnect?
//
// It seems useful, but most of the time those handlers would be used to just
// save the user state, and not for error handling (which can/should be done
// outside of Dioxus, to which `DisconnectReason` is returned to as well).
// It shouldn't matter, why the client disconnects. However there aren't many
// advantages of not providing that information.
//
// The benefits I can see are:
//
// 1. `DisconnectHandlers` and `DisconnectClientActions` could be stored in a
// single `Rc<RefCell<_>>`, instead of two separate ones (which happens only
// once at the start of the connection)
//
// 2. The `DisconnectReason` argument of the handler requires a type annotation
// because of the `SendErr` and `RecvErr` generics. So for axum someone has to
// write `|_: &AxumDisconnectReason| {..}`, instead of `|_| {..}`.
//
// While I can't really come up with a scenario where I would use a disconnect
// handler in a different way, depending of the reason of the disconnect,
// the above disadvantages don't look too bad, but let me know if you think
// differently.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClientStatus {
    /// Status during initial render when the client connects  for the first
    /// time (i.e. no client ID was assigned)
    Connects,

    /// Status during initial render when the client reconnects after the
    /// connection with the server was lost (i.e. a client ID was previously
    /// assigned)
    Reconnects,

    /// The status after the initial render (during which the status is either
    /// `Connects` or `Reconnects`)
    Initiated,
}

pub fn get_client_status(cx: &ScopeState) -> ClientStatus {
    cx.consume_context::<ClientStatus>()
        .expect("`dioxus_liveview::pool::run` should provide `ClientStatus`")
}

// XXX: `get_client_status` isn't reactive (e.g. implemented via
// `use_shared_state`) because `ClientStatus` is only intended for doing setup
// work during the first render (e.g. restoring state when reconnecting). After
// the first render, `ClientStatus` will always be `Initiated`, so reactivity
// wouldn't make sense (and just cause an additional re-render immediately
// after the first one)

/// TODO: Docs.
pub fn use_disconnect_client_actions<Actions, Iter>(cx: &ScopeState, actions: Actions)
where
    Actions: FnOnce() -> Iter,
    Iter: IntoIterator<Item = DisconnectClientAction>,
{
    cx.use_hook(move || {
        log::debug!("Storing disconnect client actions"); // TODO: Remove when example is finished

        let handlers = cx
            .consume_context::<DisconnectClientActions>()
            .expect("`DisconnectClientActions` should be provided by `pool::run`");

        let mut hh = handlers.borrow_mut();
        hh.extend(actions().into_iter());
    });
}

pub(crate) type DisconnectClientActions = Rc<RefCell<Vec<DisconnectClientAction>>>;

type CowStr = Cow<'static, str>;

// TODO: Docs
#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", content = "data")]
pub enum DisconnectClientAction {
    // /// Considered dangerous because special care is required when embedding
    // /// data that originates from untrusted sources (e.g. users of the
    // /// application). Otherwise security vulnerabilities like Cross-Site-
    // /// Scripting (XXS) might be introduced.
    // DangerouslyExecJs(CowStr),
    // XXX: See my comment below for the reason why I've uncommented the above
    //
    /// Calls a function in the global scope.
    CallJsFn(CowStr),

    /// Sets attribute `name` to `value` on every HTML element that CSS
    /// selector `selector` returns.
    // TODO: Maybe forbid dangerous attributes like `onerror`, that can lead to
    // Cross-Site-Scripting vulnerabilities.
    SetAttribute {
        selector: CowStr,
        name: CowStr,
        value: CowStr,
    },
}

// TODO: There probably also should be `ReconnectClientAction`, e.g. to revert
// a change made by a `DisconnectClientAction` that influenced something
// outside the app root element.

/* XXX: Regarding `DisconnectClientAction` and Cross-Site-Scripting (XSS):

I started implementing `DisconnectClientAction` by adding `DangerouslyExecJs`,
which basically excutes provided JavaScript code via the `eval`-like `Function`
constructor. I uncommented it because `SetAttribute` and `CallJsFn` should be
more than enough for most use-cases.

I'm not a secutity expert, however I'm currently working on becoming one, and
if I've learned anything so far, it is that it is crazy what malicious actors
can come up with.

Therefore, I don't think it is worth to increase the attack surface of Dioxus-
LiveView by including `eval`-like functionality. Even if it seems unlikely that
it can be exploited.

For similar reasons I've decided to not allow arguments to be passed to the
function executed via `CallJsFn`, which would allow the execution of `eval`.

I'd also like to disallow attributes that browsers execute as JavaScript
(like event handler, and attributes that accept JavaScript URLs, e.g. `<a
href="javascript:alert(document.domain)">`).

That being said, Dioxus seems to already include similarly dangerous
functionality, like  rendering script tags, and (if I remember correctly) event
handler string attribute, which are interpreted as JavaScript, so you might
think differently.

Let me know if you want to include `DangerouslyExecJs`, and allow `CallJsFn` to
accept arguments, etc.

People who don't like this, could still use a strict Content Security Policy
to disable `eval`-like functionality.
*/
