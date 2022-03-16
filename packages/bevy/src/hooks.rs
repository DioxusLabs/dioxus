use crate::{BevyDesktopContext, CustomUserEvent};
use dioxus_core::*;
// use dioxus_hooks::{use_future, UseFuture, UseFutureDep};
// use std::{fmt::Debug, future::Future};

pub fn use_bevy_window<CoreCommand, UICommand>(
    cx: &ScopeState,
) -> &BevyDesktopContext<CustomUserEvent<CoreCommand>, CoreCommand, UICommand>
where
    CoreCommand: Debug + Clone,
    UICommand: Clone + 'static,
{
    cx.use_hook(|_| {
        cx.consume_context::<BevyDesktopContext<CustomUserEvent<CoreCommand>, CoreCommand, UICommand>>()
    })
    .as_ref()
    .unwrap()
}

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
