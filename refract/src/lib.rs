//! # refract
//!
//! An experimental Dioxus-like reactive UI runtime built from scratch around
//! **stores and lenses**, inspired by [qk](https://github.com/ealmloff/qk).
//!
//! The [`Ui`] runtime owns the root state; reactive closures receive a
//! [`Ctx`] of split `&mut` borrows, so all aliasing rules are enforced
//! statically by the borrow checker — there is **no `RefCell`** and no
//! runtime borrow counting. Reads and writes are **zero-copy** (`&T` /
//! `&mut T` through [`Lens`] projections) and tracked via `Deref` /
//! `DerefMut` on guards, qk `RwTrack`-style. See `DESIGN.md` for the full
//! rationale, in particular the memo/resource semantics and `Pin` handling.
//!
//! ```
//! use refract::{Ui, lens};
//!
//! struct App { count: i32, step: i32 }
//!
//! let mut ui = Ui::new(App { count: 0, step: 2 });
//! let count = lens!(App => 0: count);
//! let step = lens!(App => 1: step);
//!
//! let doubled = ui.memo(move |ctx| *ctx.get(count) * 2);
//!
//! ui.with(|ctx| {
//!     let by = *ctx.peek(step);
//!     *ctx.write(count) += by;
//! });
//!
//! assert_eq!(*ui.read_memo(doubled), 4);
//! ```

#![warn(missing_docs)]

mod dom;
mod lens;
mod ui;

pub use dom::{Dom, El, Node, NodeId, dyn_text, el, mount, text};
pub use lens::{Field, Index, Lens, Path, Root, VecLens};
pub use ui::{Ctx, Effect, Memo, ReadGuard, ResourceHandle, ResourceState, Ui, WriteGuard};
