//! Path addressing for optic accessors.
//!
//! The `Pathed` trait is orthogonal to [`Access`](crate::Access): plain
//! signals and other accessors don't need to implement it, and their reads
//! and writes never touch it. Only the subscription wrapper
//! [`Subscribed`](crate::Subscribed) consults `Pathed` to derive
//! path-granular subscriptions anywhere in an optic chain.
//!
//! A segment is just an opaque identifier for "which child of the parent
//! this op selects." Field lenses, prisms, collection indices, and map
//! keys all use the same [`PathSegment`] type — the subscription tree
//! only needs same-child-or-not equality, not the flavor of selection.

use std::any::TypeId;
use std::hash::{DefaultHasher, Hash, Hasher};

/// Opaque child-of-parent identifier used as one layer of a subscription path.
///
/// Structurally a `u64` (fn-pointer hash, `TypeId` hash, integer index, or
/// hashed key). Equality + hashability are all the subscription tree needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PathSegment(pub u64);

impl PathSegment {
    /// Construct a segment from any hashable identifier.
    #[inline]
    pub fn hashed<Q: ?Sized + Hash>(key: &Q) -> Self {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        PathSegment(hasher.finish())
    }

    /// Construct a segment from a raw integer (e.g. a `Vec` index).
    #[inline]
    pub const fn index(i: u64) -> Self {
        PathSegment(i)
    }

    /// Segment identifying a field lens by its `fn` pointer.
    #[inline]
    pub fn field_fn<T, U>(read: fn(&T) -> &U) -> Self {
        PathSegment((read as *const () as usize) as u64)
    }

    /// Segment identifying a prism variant by its type identity.
    #[inline]
    pub fn prism_type<P: 'static>() -> Self {
        let mut hasher = DefaultHasher::new();
        TypeId::of::<P>().hash(&mut hasher);
        PathSegment(hasher.finish())
    }
}

/// Collector that accumulates [`PathSegment`]s as an optic chain is walked.
#[derive(Default, Clone)]
pub struct PathBuffer {
    segments: Vec<PathSegment>,
}

impl PathBuffer {
    /// Create an empty path buffer.
    pub fn new() -> Self {
        Self { segments: Vec::new() }
    }

    /// Append one segment.
    pub fn push(&mut self, segment: PathSegment) {
        self.segments.push(segment);
    }

    /// Borrow the accumulated path.
    pub fn segments(&self) -> &[PathSegment] {
        &self.segments
    }

    /// Reset the buffer without deallocating its backing storage.
    pub fn clear(&mut self) {
        self.segments.clear();
    }
}

/// An accessor that can describe the path it represents.
///
/// This is a separate capability from [`Access`](crate::Access): normal
/// signals and root accessors don't compute or store paths. Only accessors
/// that participate in path-granular subscription (via
/// [`Subscribed`](crate::Subscribed)) need to implement `Pathed`.
pub trait Pathed {
    /// Append this accessor's path segments to `sink`, in root-to-leaf
    /// order. Implementors must first delegate to their parent so segments
    /// accumulate in the right order.
    fn visit_path(&self, sink: &mut PathBuffer);
}
