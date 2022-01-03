mod usestate;
pub use usestate::{use_state, UseState};

mod useref;
pub use useref::*;

mod use_shared_state;
pub use use_shared_state::*;

mod usecoroutine;
pub use usecoroutine::*;

mod usefuture;
pub use usefuture::*;

mod usesuspense;
pub use usesuspense::*;

// #[macro_export]
// macro_rules! to_owned {
//     ($($es:ident),+) => {$(
//         #[allow(unused_mut)]
//         let mut $es = $es.to_owned();
//     )*}
// }

// /// Calls `for_async` on the series of paramters.
// ///
// /// If the type is Clone, then it will be cloned. However, if the type is not `clone`
// /// then it must have a `for_async` method for Rust to lower down into.
// ///
// /// See: how use_state implements `for_async` but *not* through the trait.
// #[macro_export]
// macro_rules! for_async {
//     ($($es:ident),+) => {$(
//         #[allow(unused_mut)]
//         let mut $es = $es.for_async();
//     )*}
// }

// /// This is a marker trait that uses decoherence.
// ///
// /// It is *not* meant for hooks to actually implement, but rather defer to their
// /// underlying implementation if they *don't* implement the trait.
// ///
// ///
// pub trait AsyncHook {
//     type Output;
//     fn for_async(self) -> Self::Output;
// }

// impl<T> AsyncHook for T
// where
//     T: ToOwned<Owned = T>,
// {
//     type Output = T;
//     fn for_async(self) -> Self::Output {
//         self
//     }
// }
