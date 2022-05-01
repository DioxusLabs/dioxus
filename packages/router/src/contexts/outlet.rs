/// A context used by outlets to determine how deep they are.
#[derive(Clone)]
pub(crate) struct OutletContext {
    /// The depth of the outlet providing the context.
    pub(crate) depth: usize,
}
