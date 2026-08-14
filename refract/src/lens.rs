//! Lenses: zero-copy, `Copy` projections from a root state type `S` down to a
//! field (or collection element). A lens is nothing but a pair of pure `fn`
//! pointers plus a structural path used for granular invalidation. Lenses
//! never own or borrow data themselves — they are applied to a `&S`/`&mut S`
//! that the runtime hands to your closures, so exclusivity is enforced
//! entirely by the borrow checker. No `RefCell`, no runtime borrow counting.

/// A structural path into the state tree. Paths are compared by prefix
/// overlap: writing `state.todos` invalidates readers of `state.todos[3].done`
/// and vice versa, but writing `state.todos[3]` does not touch readers of
/// `state.todos[4]`.
pub type Path = Vec<u32>;

/// `true` if one path is a prefix of the other.
pub(crate) fn paths_overlap(a: &[u32], b: &[u32]) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| x == y)
}

/// A composable projection from root state `S` to a target `T`.
///
/// Implementors are `Copy` handles: [`Root`], [`Field`], and [`Index`].
pub trait Lens<S: 'static>: Copy + 'static {
    /// The projected target type.
    type Target: ?Sized + 'static;

    /// Append this lens' structural path segments.
    fn push_path(&self, out: &mut Path);

    /// Project a shared reference.
    fn get<'a>(&self, state: &'a S) -> &'a Self::Target;

    /// Project a mutable reference.
    fn get_mut<'a>(&self, state: &'a mut S) -> &'a mut Self::Target;

    /// The full structural path of this lens.
    fn path(&self) -> Path {
        let mut out = Path::new();
        self.push_path(&mut out);
        out
    }

    /// Compose with a field projection. Prefer the [`lens!`](crate::lens!)
    /// macro, which derives the segment index and both `fn` pointers.
    fn field<T: ?Sized + 'static>(
        self,
        segment: u32,
        get: fn(&Self::Target) -> &T,
        get_mut: fn(&mut Self::Target) -> &mut T,
    ) -> Field<Self, Self::Target, T> {
        Field {
            parent: self,
            segment,
            get,
            get_mut,
        }
    }
}

/// The identity lens: projects `S` to itself.
pub struct Root<S>(std::marker::PhantomData<fn() -> S>);

impl<S> Root<S> {
    /// Create the identity lens for `S`.
    pub fn new() -> Self {
        Root(std::marker::PhantomData)
    }
}

impl<S> Default for Root<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Clone for Root<S> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<S> Copy for Root<S> {}

impl<S: 'static> Lens<S> for Root<S> {
    type Target = S;

    fn push_path(&self, _out: &mut Path) {}

    fn get<'a>(&self, state: &'a S) -> &'a S {
        state
    }

    fn get_mut<'a>(&self, state: &'a mut S) -> &'a mut S {
        state
    }
}

/// A field projection composed over a parent lens whose target is `U`.
pub struct Field<P, U: ?Sized, T: ?Sized> {
    parent: P,
    segment: u32,
    get: fn(&U) -> &T,
    get_mut: fn(&mut U) -> &mut T,
}

impl<P: Copy, U: ?Sized, T: ?Sized> Clone for Field<P, U, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<P: Copy, U: ?Sized, T: ?Sized> Copy for Field<P, U, T> {}

impl<S, P, U, T> Lens<S> for Field<P, U, T>
where
    S: 'static,
    P: Lens<S, Target = U>,
    U: ?Sized + 'static,
    T: ?Sized + 'static,
{
    type Target = T;

    fn push_path(&self, out: &mut Path) {
        self.parent.push_path(out);
        out.push(self.segment);
    }

    fn get<'a>(&self, state: &'a S) -> &'a T {
        (self.get)(self.parent.get(state))
    }

    fn get_mut<'a>(&self, state: &'a mut S) -> &'a mut T {
        (self.get_mut)(self.parent.get_mut(state))
    }
}

/// An element projection into a `Vec<T>`, carrying a runtime index.
pub struct Index<P, T> {
    parent: P,
    index: usize,
    _marker: std::marker::PhantomData<fn() -> T>,
}

impl<P: Copy, T> Clone for Index<P, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<P: Copy, T> Copy for Index<P, T> {}

impl<S, P, T> Lens<S> for Index<P, T>
where
    S: 'static,
    P: Lens<S, Target = Vec<T>>,
    T: 'static,
{
    type Target = T;

    fn push_path(&self, out: &mut Path) {
        self.parent.push_path(out);
        out.push(self.index as u32);
    }

    fn get<'a>(&self, state: &'a S) -> &'a T {
        &self.parent.get(state)[self.index]
    }

    fn get_mut<'a>(&self, state: &'a mut S) -> &'a mut T {
        &mut self.parent.get_mut(state)[self.index]
    }
}

/// Extension for lenses over `Vec<T>`: project to an element.
pub trait VecLens<S: 'static, T: 'static>: Lens<S, Target = Vec<T>> {
    /// Lens to element `index`. Writing one element does not invalidate
    /// readers of its siblings.
    fn at(self, index: usize) -> Index<Self, T> {
        Index {
            parent: self,
            index,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S: 'static, T: 'static, L: Lens<S, Target = Vec<T>>> VecLens<S, T> for L {}

/// Build a field lens from a chain of `segment: field` pairs.
///
/// ```ignore
/// let name = lens!(App => 0: user, 1: name);
/// ```
///
/// Segment indices must be distinct per struct level; they define the
/// structural path used for granular invalidation.
#[macro_export]
macro_rules! lens {
    ($root:ty => $($segment:literal : $field:ident),+ $(,)?) => {{
        let l = $crate::Root::<$root>::new();
        $(
            let l = $crate::Lens::<$root>::field(
                l,
                $segment,
                |s| &s.$field,
                |s| &mut s.$field,
            );
        )+
        l
    }};
}
