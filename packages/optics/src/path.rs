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

/// Maximum optic-chain depth supported by [`PathBuffer`].
///
/// Picked to match the inline-storage size of the old `dioxus-stores`
/// `TinyVec`. A chain deeper than this panics on `PathBuffer::push`.
pub const PATH_LEN: usize = 32;

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

/// Stack-allocated, fixed-capacity collector that accumulates
/// [`PathSegment`]s as an optic chain is walked.
///
/// `Copy`, no heap allocation, capacity [`PATH_LEN`]. Modeled after the old
/// `dioxus-stores` `TinyVec` so optic mappings don't pay a per-read alloc
/// and don't require any owner-bound storage to subscribe.
#[derive(Clone, Copy)]
pub struct PathBuffer {
    len: u8,
    segments: [PathSegment; PATH_LEN],
}

impl Default for PathBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl PathBuffer {
    /// Create an empty path buffer.
    #[inline]
    pub const fn new() -> Self {
        Self {
            len: 0,
            segments: [PathSegment(0); PATH_LEN],
        }
    }

    /// Append one segment. Panics if the buffer is already at [`PATH_LEN`].
    #[inline]
    pub const fn push(&mut self, segment: PathSegment) {
        assert!(
            (self.len as usize) < PATH_LEN,
            "optics: path depth exceeded PATH_LEN",
        );
        self.segments[self.len as usize] = segment;
        self.len += 1;
    }

    /// Borrow the accumulated path as a slice.
    #[inline]
    pub fn segments(&self) -> &[PathSegment] {
        &self.segments[..self.len as usize]
    }

    /// Number of segments currently stored.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Returns `true` if the buffer holds no segments.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Reset the buffer.
    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl std::fmt::Debug for PathBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.segments().iter()).finish()
    }
}

impl PartialEq for PathBuffer {
    fn eq(&self, other: &Self) -> bool {
        self.segments() == other.segments()
    }
}

impl Eq for PathBuffer {}

impl Hash for PathBuffer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.segments().hash(state);
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
