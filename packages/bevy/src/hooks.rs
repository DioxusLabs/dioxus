use crate::context::DesktopContext;
use dioxus_core::*;
use std::fmt::Debug;

pub fn use_bevy_window<CoreCommand, UICommand>(
    cx: &ScopeState,
) -> &DesktopContext<CoreCommand, UICommand>
where
    CoreCommand: Debug + Clone,
    UICommand: Clone + 'static,
{
    cx.use_hook(|_| cx.consume_context::<DesktopContext<CoreCommand, UICommand>>())
        .as_ref()
        .unwrap()
}

// TODO
// pub fn use_bevy_listener<CoreCommand, UICommand, D, F>(
//     cx: &ScopeState,
//     deps: D,
//     handler: impl Fn(UICommand, D::Out) -> F + 'static,
// ) -> &UseFuture<()>
// where
//     CoreCommand: Debug + Clone + 'static,
//     UICommand: Clone + 'static,
//     F: Future<Output = ()> + 'static,
//     D: UseFutureDep + 'static,
//     <D as UseFutureDep>::Out: Clone,
// {
//     let ctx = use_bevy_window::<CoreCommand, UICommand>(&cx);

//     let state = use_future(&cx, deps, |deps| {
//         let mut rx = ctx.receiver();

//         async move {
//             while let Ok(cmd) = rx.recv().await {
//                 handler(cmd, deps);
//             }
//         }
//     });

//     state
// }
