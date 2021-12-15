// use dioxus_core::prelude::Context;
// use std::{
//     borrow::{Borrow, BorrowMut},
//     cell::{Cell, Ref, RefCell, RefMut},
//     fmt::{Debug, Display},
//     ops::Not,
//     rc::Rc,
// };

// /// Store state between component renders!
// ///
// /// ## Dioxus equivalent of UseStateInner2, designed for Rust
// ///
// /// The Dioxus version of `UseStateInner2` is the "king daddy" of state management. It allows you to ergonomically store and
// /// modify state between component renders. When the state is updated, the component will re-render.
// ///
// /// Dioxus' use_state basically wraps a RefCell with helper methods and integrates it with the VirtualDOM update system.
// ///
// /// [`use_state`] exposes a few helper methods to modify the underlying state:
// /// - `.set(new)` allows you to override the "work in progress" value with a new value
// /// - `.get_mut()` allows you to modify the WIP value
// /// - `.get_wip()` allows you to access the WIP value
// /// - `.deref()` provides the previous value (often done implicitly, though a manual dereference with `*` might be required)
// ///
// /// Additionally, a ton of std::ops traits are implemented for the `UseStateInner2` wrapper, meaning any mutative type operations
// /// will automatically be called on the WIP value.
// ///
// /// ## Combinators
// ///
// /// On top of the methods to set/get state, `use_state` also supports fancy combinators to extend its functionality:
// /// - `.classic()` and `.split()`  convert the hook into the classic React-style hook
// ///     ```rust
// ///     let (state, set_state) = use_state(cx, || 10).split()
// ///     ```
// ///
// ///
// /// Usage:
// /// ```ignore
// /// const Example: FC<()> = |cx, props|{
// ///     let counter = use_state(cx, || 0);
// ///     let increment = |_| counter += 1;
// ///     let decrement = |_| counter += 1;
// ///
// ///     html! {
// ///         <div>
// ///             <h1>"Counter: {counter}" </h1>
// ///             <button onclick={increment}> "Increment" </button>
// ///             <button onclick={decrement}> "Decrement" </button>
// ///         </div>
// ///     }
// /// }
// /// ```
// pub fn use_state2<'a, T: 'static>(
//     cx: Context<'a>,
//     initial_state_fn: impl FnOnce() -> T,
// ) -> &'a UseState2<T> {
//     cx.use_hook(
//         move |_| {
//             UseState2(Rc::new(UseStateInner2 {
//                 current_val: initial_state_fn(),
//                 update_callback: cx.schedule_update(),
//                 wip: None,
//                 update_scheuled: Cell::new(false),
//             }))
//         },
//         move |hook: &mut UseState2<T>| {
//             {
//                 let r = hook.0.as_ref();
//                 let mut state = r.borrow_mut();
//                 state.update_scheuled.set(false);
//                 if state.wip.is_some() {
//                     state.current_val = state.wip.take().unwrap();
//                 }
//             }
//             &*hook
//         },
//     )
// }

// pub struct UseState2<T: 'static>(Rc<UseStateInner2<T>>);

// impl<T> ToOwned for UseState2<T> {
//     type Owned = UseState2<T>;
//     fn to_owned(&self) -> Self::Owned {
//         UseState2(self.0.clone())
//     }
// }

// pub struct UseStateInner2<T: 'static> {
//     current_val: T,
//     update_scheuled: Cell<bool>,
//     update_callback: Rc<dyn Fn()>,
//     wip: Option<T>,
// }

// impl<T: Debug> Debug for UseStateInner2<T> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{:?}", self.current_val)
//     }
// }

// impl<T> UseState2<T> {
//     /// Tell the Dioxus Scheduler that we need to be processed
//     pub fn needs_update(&self) {
//         if !self.update_scheuled.get() {
//             self.update_scheuled.set(true);
//             (self.update_callback)();
//         }
//     }

//     pub fn set(&mut self, new_val: T) -> Option<T> {
//         self.needs_update();
//         ip.wip.replace(new_val)
//     }

//     pub fn get(&self) -> &T {
//         &self.current_val
//     }
// }

// // impl<T: 'static + ToOwned<Owned = T>> UseState2<T> {
// //     /// Gain mutable access to the new value. This method is only available when the value is a `ToOwned` type.
// //     ///
// //     /// Mutable access is derived by calling "ToOwned" (IE cloning) on the current value.
// //     ///
// //     /// To get a reference to the current value, use `.get()`
// //     pub fn modify(&self) -> RefMut<T> {
// //         // make sure we get processed
// //         self.0.needs_update();

// //         // Bring out the new value, cloning if it we need to
// //         // "get_mut" is locked behind "ToOwned" to make it explicit that cloning occurs to use this
// //         RefMut::map(self.wip.borrow_mut(), |slot| {
// //             if slot.is_none() {
// //                 *slot = Some(self.current_val.to_owned());
// //             }
// //             slot.as_mut().unwrap()
// //         })
// //     }

// //     pub fn inner(self) -> T {
// //         self.current_val.to_owned()
// //     }
// // }

// impl<T> std::ops::Deref for UseStateInner2<T> {
//     type Target = T;

//     fn deref(&self) -> &Self::Target {
//         self.get()
//     }
// }

// use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

// use crate::UseState;

// impl<T: Copy + Add<T, Output = T>> Add<T> for UseStateInner2<T> {
//     type Output = T;

//     fn add(self, rhs: T) -> Self::Output {
//         self.current_val.add(rhs)
//     }
// }
// impl<T: Copy + Add<T, Output = T>> AddAssign<T> for UseStateInner2<T> {
//     fn add_assign(&mut self, rhs: T) {
//         self.set(self.current_val.add(rhs));
//     }
// }
// impl<T: Copy + Sub<T, Output = T>> Sub<T> for UseStateInner2<T> {
//     type Output = T;

//     fn sub(self, rhs: T) -> Self::Output {
//         self.current_val.sub(rhs)
//     }
// }
// impl<T: Copy + Sub<T, Output = T>> SubAssign<T> for UseStateInner2<T> {
//     fn sub_assign(&mut self, rhs: T) {
//         self.set(self.current_val.sub(rhs));
//     }
// }

// /// MUL
// impl<T: Copy + Mul<T, Output = T>> Mul<T> for UseStateInner2<T> {
//     type Output = T;

//     fn mul(self, rhs: T) -> Self::Output {
//         self.current_val.mul(rhs)
//     }
// }
// impl<T: Copy + Mul<T, Output = T>> MulAssign<T> for UseStateInner2<T> {
//     fn mul_assign(&mut self, rhs: T) {
//         self.set(self.current_val.mul(rhs));
//     }
// }
// /// DIV
// impl<T: Copy + Div<T, Output = T>> Div<T> for UseStateInner2<T> {
//     type Output = T;

//     fn div(self, rhs: T) -> Self::Output {
//         self.current_val.div(rhs)
//     }
// }
// impl<T: Copy + Div<T, Output = T>> DivAssign<T> for UseStateInner2<T> {
//     fn div_assign(&mut self, rhs: T) {
//         self.set(self.current_val.div(rhs));
//     }
// }
// impl<V, T: PartialEq<V>> PartialEq<V> for UseStateInner2<T> {
//     fn eq(&self, other: &V) -> bool {
//         self.get() == other
//     }
// }
// impl<O, T: Not<Output = O> + Copy> Not for UseStateInner2<T> {
//     type Output = O;

//     fn not(self) -> Self::Output {
//         !*self.get()
//     }
// }

// // enable displaty for the handle
// impl<T: 'static + Display> std::fmt::Display for UseStateInner2<T> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}", self.current_val)
//     }
// }
