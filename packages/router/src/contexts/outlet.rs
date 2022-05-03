use std::collections::BTreeMap;

/// A context used by outlets to determine how deep they are.
#[derive(Clone)]
pub(crate) struct OutletContext {
    /// The depth of the outlet providing the context.
    pub(crate) depth: Option<usize>,
    /// Same as `depth` but for named outlets.
    pub(crate) named_depth: BTreeMap<String, usize>,
}
