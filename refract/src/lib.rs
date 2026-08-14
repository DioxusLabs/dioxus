//! # Refract
//!
//! A from-scratch, store/lens-first reactive UI runtime in the spirit of
//! Dioxus, inspired by [ealmloff/qk](https://github.com/ealmloff/qk)'s
//! `Deref`/`DerefMut` dirty tracking. See `DESIGN.md` for the full design
//! discussion (especially the memo/resource borrow-safety story).
//!
//! ```
//! use refract::prelude::*;
//!
//! #[derive(PartialEq)]
//! struct App { count: i32, step: i32 }
//!
//! let app = Store::new(App { count: 0, step: 2 });
//! let count = lens!(app => 0: count);
//! let step = lens!(app => 1: step);
//!
//! let doubled = memo(move || *count.read() * 2);
//!
//! let view = el("div").child(dyn_text(move || format!("{}", doubled.read())));
//!
//! // Lenses over one store share its cell: finish reads before writing.
//! let by = *step.peek();
//! *count.write() += by;
//! assert_eq!(view.render_to_string(), "<div>4</div>");
//! ```

mod dom;
mod effect;
mod memo;
mod resource;
mod runtime;
mod store;

pub use dom::{Element, dyn_text, el, text};
pub use effect::{Effect, effect};
pub use memo::{Memo, memo};
pub use resource::{Resource, ResourceState, resource};
pub use runtime::{NodeId, flush, run_until_settled};
pub use store::{IndexLens, Lens, ReadGuard, Readable, Store, Writable, WriteGuard, untracked};

pub mod prelude {
    pub use crate::dom::{Element, dyn_text, el, text};
    pub use crate::effect::{Effect, effect};
    pub use crate::lens;
    pub use crate::memo::{Memo, memo};
    pub use crate::resource::{Resource, ResourceState, resource};
    pub use crate::runtime::{flush, run_until_settled};
    pub use crate::store::{Readable, Store, Writable, untracked};
}
