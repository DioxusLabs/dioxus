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
    Any, BTreeMapKey, Cloned, CollectionLen, EachBTreeMap, EachHashMap, EachVec, FlattenSome,
    FlattenSomeOp, GetProjection, HashMapKey, IsEmpty, Len, OpticBTreeMapExt, OpticHashMapExt,
    OpticVecExt, Position, Values, VecIndex,
};
pub use combinator::{
    Access, AccessMut, Combinator, ErrPrism, FutureAccess, InlinePrism, LensOp, OkPrism,
    OptPrismOp, Prism, PrismOp, RefOp, Resolve, SomePrism, ValueAccess,
};
#[doc(hidden)]
pub use ext::BorrowProject;
pub use ext::{OpticExt, OpticMutExt, OpticRefExt};
pub use iter::{IterShape, OpticIter};
pub use path::{PathBuffer, PathSegment, Pathed, PATH_LEN};
pub use resource::{AsFuture, AwaitTransform, FutureProject, Resource, ResourceFuture};
pub use signal::{Optic, Optional, Required};
pub use subscribed::{HasSubscriptionTree, Subscribed, SubscriptionTree};

/// Common imports for the experimental optics API.
pub mod prelude {
    pub use crate::{
        Access, AccessMut, Any, AsFuture, AwaitTransform, BTreeMapKey, Cloned, Combinator,
        EachBTreeMap, EachHashMap, EachVec, ErrPrism, FlattenSome, FlattenSomeOp, FutureAccess,
        FutureProject, GetProjection, HasSubscriptionTree, HashMapKey, InlinePrism, IsEmpty,
        IterShape, Len, LensOp, OkPrism, OptPrismOp, Optic, OpticBTreeMapExt, OpticExt,
        OpticHashMapExt, OpticIter, OpticMutExt, OpticRefExt, OpticVecExt, Optional, PathBuffer,
        PathSegment, Pathed, Position, Prism, PrismOp, RefOp, Required, Resolve, Resource,
        ResourceFuture, SomePrism, Subscribed, SubscriptionTree, ValueAccess, Values, VecIndex,
        PATH_LEN,
    };
}
