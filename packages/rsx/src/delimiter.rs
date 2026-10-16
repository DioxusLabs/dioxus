use proc_macro2::Span;
use syn::{Token, token::Brace};

/// The tokens that delimit the body of an [`Element`](crate::Element) or
/// [`Component`](crate::Component)
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum NodeDelimiter {
    /// The regular block syntax: `div { }`
    Brace(Brace),

    /// The JSX/XML-like tag syntax: `<div></div>` or `<div />`
    Tag(TagDelimiter),

    /// The node has no body yet: `div`
    ///
    /// This is not valid rsx, but is parsed (with a diagnostic) so that completions keep working
    /// while the node is being typed
    Missing,
}

impl NodeDelimiter {
    pub fn brace(&self) -> Option<&Brace> {
        match self {
            Self::Brace(brace) => Some(brace),
            _ => None,
        }
    }

    pub fn tag(&self) -> Option<&TagDelimiter> {
        match self {
            Self::Tag(tag) => Some(tag),
            _ => None,
        }
    }

    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }

    /// The span of the token that starts the body of the node: the `{` of a block or the `>` that
    /// ends an open tag
    pub fn open_span(&self) -> Option<Span> {
        match self {
            Self::Brace(brace) => Some(brace.span.open()),
            Self::Tag(tag) => tag.gt.map(|gt| gt.span),
            Self::Missing => None,
        }
    }

    /// The span of the last token of the node: the `}` of a block or the final `>` of a tag
    pub fn close_span(&self) -> Option<Span> {
        match self {
            Self::Brace(brace) => Some(brace.span.close()),
            Self::Tag(tag) => match &tag.close {
                Some(close) => Some(close.gt.span),
                None => tag.gt.map(|gt| gt.span),
            },
            Self::Missing => None,
        }
    }
}

impl From<Brace> for NodeDelimiter {
    fn from(brace: Brace) -> Self {
        Self::Brace(brace)
    }
}

/// The tokens of a node written in the tag syntax
///
/// ```text
/// <div class="a"> "child" </div>      <img src="a.png" />
/// ^             ^         ^^^^^^      ^                ^^
/// lt            gt        close       lt            slash, gt
/// ```
///
/// A complete tag is either self-closing (`slash` and `gt` are set) or has a closing tag (`gt`
/// and `close` are set). Tags that are still being typed are parsed with a diagnostic and may be
/// missing either.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct TagDelimiter {
    /// The `<` that starts the open tag
    pub lt: Token![<],

    /// The `/` of a self-closing tag: `<div />`
    pub slash: Option<Token![/]>,

    /// The `>` that ends the open tag
    pub gt: Option<Token![>]>,

    /// The closing tag: `</div>`
    pub close: Option<ClosingTag>,
}

impl TagDelimiter {
    /// `<div />`
    pub fn is_self_closing(&self) -> bool {
        self.slash.is_some() && self.gt.is_some()
    }

    /// Whether the tag was fully written out: either self-closing or with a closing tag
    pub fn is_complete(&self) -> bool {
        self.is_self_closing() || (self.gt.is_some() && self.close.is_some())
    }
}

/// The tokens of a closing tag: `</div>`
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ClosingTag {
    pub lt: Token![<],
    pub slash: Token![/],
    pub gt: Token![>],
}
