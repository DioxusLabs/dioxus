#![doc = include_str!("../README.md")]

mod collection;
mod combinator;
mod resource;
mod signal;

pub use collection::{
    BTreeMapGet, BTreeMapKey, EachBTreeMap, EachHashMap, EachVec, FlattenSome, FlattenSomeOp,
    HashMapGet, HashMapKey, VecGet, VecIndex,
};
pub use combinator::{
    Combinator, FutureProjection, LensOp, ReadProjection, ReadProjectionOpt, Resolve, Transform,
    UnwrapErrOp, UnwrapErrOptionalOp, UnwrapOkOp, UnwrapOkOptionalOp, UnwrapSomeOp,
    UnwrapSomeOptionalOp, ValueProjection, WriteProjection, WriteProjectionOpt,
};
pub use resource::{AsFuture, AwaitTransform, FutureProject, Resource, ResourceFuture};
pub use signal::{Optic, Optional, Required, RwRoot};

/// Common imports for the experimental optics API.
pub mod prelude {
    pub use crate::{
        AsFuture, AwaitTransform, FutureProject, FutureProjection, Optic, Optional, ReadProjection,
        ReadProjectionOpt, Required, Resource, ResourceFuture, RwRoot, ValueProjection,
        WriteProjection, WriteProjectionOpt,
    };
}
