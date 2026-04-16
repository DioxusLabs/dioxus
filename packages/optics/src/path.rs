//! Path addressing for optic accessors.
//!
//! The `Pathed` trait is orthogonal to [`Access`](crate::Access): plain
//! signals and other accessors don't need to implement it, and their reads
//! and writes never touch it. Only the subscription wrapper
//! [`Subscribed`](crate::Subscribed) consults `Pathed` to derive
//! path-granular subscriptions anywhere in an optic chain.
//!
//! An op appends one [`PathSegment`] for each layer it adds. Field lenses
//! use their `fn` pointer as a stable, build-local identity; prisms use the
//! prism type's `TypeId`; collection indices use their integer key; map
//! keys use a precomputed hash.

use std::any::TypeId;
use std::hash::{DefaultHasher, Hash, Hasher};

/// One layer in the path an accessor chain represents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PathSegment {
    /// Field projection identified by a stable hash.
    Field(u64),
    /// Variant projection (prism) identified by the prism type's `TypeId`.
    Variant(u64),
    /// Numeric index into a `Vec`-like carrier.
    Index(u64),
    /// Hashable key into a map-like carrier.
    Key(u64),
    /// Flatten-like op (`flatten_some`).
    Flatten,
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

/// Hash a field's `fn` pointer identity into a 64-bit segment key.
#[inline]
pub fn hash_field_fn<T, U>(read: fn(&T) -> &U) -> u64 {
    (read as *const () as usize) as u64
}

/// Hash any hashable key (HashMap/BTreeMap keys) into a 64-bit segment key.
#[inline]
pub fn hash_key<Q: ?Sized + Hash>(key: &Q) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

/// Hash a prism's [`TypeId`] into a 64-bit segment key.
#[inline]
pub fn hash_prism_type<P: 'static>() -> u64 {
    let mut hasher = DefaultHasher::new();
    TypeId::of::<P>().hash(&mut hasher);
    hasher.finish()
}
