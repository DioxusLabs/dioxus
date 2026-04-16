#![doc = include_str!("../README.md")]

mod collection;
mod combinator;
mod resource;
mod signal;

pub use collection::{
    BTreeMapKey, EachBTreeMap, EachHashMap, EachVec, FlattenSome, FlattenSomeOp, GetProjection,
    HashMapKey, VecIndex,
};
pub use combinator::{
    Access, AccessMut, Combinator, ErrPrism, FutureAccess, InlinePrism, LensOp, OkPrism,
    OptPrismOp, Prism, PrismOp, ReadCarrier, RefOp, Resolve, SomePrism, ValueAccess, WriteCarrier,
};
pub use resource::{AsFuture, AwaitTransform, FutureProject, Resource, ResourceFuture};
pub use signal::{Optic, Optional, Required};

/// Common imports for the experimental optics API.
pub mod prelude {
    pub use crate::{
        Access, AccessMut, AsFuture, AwaitTransform, FutureAccess, FutureProject, InlinePrism,
        Optic, Optional, Prism, Required, Resource, ResourceFuture, SomePrism, ValueAccess,
    };
}
