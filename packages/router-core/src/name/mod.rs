use std::{
    any::{type_name, TypeId},
    fmt::Display,
    hash::Hash,
};

pub(crate) mod segments;

/// A combination of a types [`TypeId`] and its name.
///
/// This is used inside the router wherever a name is needed. This has the advantage that typos will
/// be caught by the compiler.
///
/// **Note:** The dioxus-router-core documentation and tests mostly use standard Rust types. This is only for
/// brevity. It is recommend to use types with descriptive names, and create unit structs if needed,
/// like this.
///
/// ```rust
/// # use dioxus_router_core::Name;
/// struct SomeName;
/// let name = Name::of::<bool>();
/// ```
#[derive(Clone, Debug)]
pub struct Name {
    id: TypeId,
    name: &'static str,
}

impl Name {
    /// Get the [`Name`] of `T`.
    ///
    /// ```rust
    /// # use dioxus_router_core::Name;
    /// struct SomeName;
    /// let name = Name::of::<bool>();
    /// ```
    #[must_use]
    pub fn of<T: 'static>() -> Self {
        Self {
            id: TypeId::of::<T>(),
            name: type_name::<T>(),
        }
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Name {}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl Ord for Name {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Hash for Name {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
