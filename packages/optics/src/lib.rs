#![doc = include_str!("../README.md")]

mod collection;
mod combinator;
mod path;
mod resource;
mod signal;
mod subscribed;

pub use collection::{
    BTreeMapKey, EachBTreeMap, EachHashMap, EachVec, FlattenSome, FlattenSomeOp, GetProjection,
    HashMapKey, VecIndex,
};
pub use combinator::{
    Access, AccessMut, Combinator, ErrPrism, FutureAccess, InlinePrism, LensOp, OkPrism,
    OptPrismOp, Prism, PrismOp, RefOp, Resolve, SomePrism, ValueAccess,
};
pub use path::{hash_field_fn, hash_key, hash_prism_type, PathBuffer, PathSegment, Pathed};
pub use resource::{AsFuture, AwaitTransform, FutureProject, Resource, ResourceFuture};
pub use signal::{Optic, Optional, Required};
pub use subscribed::{Subscribed, SubscriptionTree};

/// Common imports for the experimental optics API.
pub mod prelude {
    pub use crate::{
        Access, AccessMut, AsFuture, AwaitTransform, FutureAccess, FutureProject, InlinePrism,
        Optic, Optional, Pathed, Prism, Required, Resource, ResourceFuture, SomePrism, Subscribed,
        SubscriptionTree, ValueAccess,
    };
}
