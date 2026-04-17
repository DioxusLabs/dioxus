#![doc = include_str!("../README.md")]

mod collection;
mod combinator;
mod ext;
mod iter;
mod path;
mod resource;
mod signal;
mod subscribed;

pub use collection::{
    Any, BTreeMapKey, EachBTreeMap, EachHashMap, EachVec, FlattenSome, FlattenSomeOp,
    GetProjection, HashMapKey, Values, VecIndex,
};
pub use combinator::{
    Access, AccessMut, Combinator, ErrPrism, FutureAccess, InlinePrism, LensOp, OkPrism,
    OptPrismOp, Prism, PrismOp, RefOp, Resolve, SomePrism, ValueAccess,
};
pub use ext::{OpticExt, OpticMutExt};
pub use iter::{IterShape, OpticIter};
pub use path::{PathBuffer, PathSegment, Pathed, PATH_LEN};
pub use resource::{AsFuture, AwaitTransform, FutureProject, Resource, ResourceFuture};
pub use signal::{Optic, Optional, Required};
pub use subscribed::{HasSubscriptionTree, Subscribed, SubscriptionTree};

/// Common imports for the experimental optics API.
pub mod prelude {
    pub use crate::{
        Access, AccessMut, AsFuture, AwaitTransform, FutureAccess, FutureProject,
        HasSubscriptionTree, InlinePrism, Optic, OpticExt, OpticIter, OpticMutExt, Optional,
        Pathed, Prism, Required, Resource, ResourceFuture, SomePrism, Subscribed, SubscriptionTree,
        ValueAccess,
    };
}
