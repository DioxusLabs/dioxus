use std::hash::Hash;

use proc_macro2::Span;

// A form of whitespace
#[derive(Debug, Clone)]
pub struct UserComment {
    pub span: Span,
    pub comment: String,
}

impl PartialEq for UserComment {
    fn eq(&self, other: &Self) -> bool {
        self.comment == other.comment
    }
}

impl Eq for UserComment {}

impl Hash for UserComment {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.comment.hash(state);
    }
}
