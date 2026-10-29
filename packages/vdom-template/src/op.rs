/// One operation in a flat static template tape.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct TemplateOp(u16);

impl TemplateOp {
    const ENTER_MAX_CODE: u16 = 0x7fff;
    const ATTR_CODE: u16 = 0x8000;
    const ATTR_CUSTOM_NS_CODE: u16 = 0x8001;
    const TEXT_CODE: u16 = 0x8002;
    const STATIC_BASE: u16 = 0x8003;
    pub(crate) const MAX_CAP: usize = 16_383;

    /// Create a packed enter op.
    pub(crate) const fn enter(skip: u16, namespace: bool) -> Self {
        if skip as usize > Self::MAX_CAP {
            panic!("op skip exceeds packed op capacity");
        }
        Self((skip << 1) | namespace as u16)
    }

    /// Create a packed static attribute op.
    pub(crate) const fn attr(namespace: bool) -> Self {
        if namespace {
            Self(Self::ATTR_CUSTOM_NS_CODE)
        } else {
            Self(Self::ATTR_CODE)
        }
    }

    /// Create a packed text marker op.
    pub(crate) const fn text() -> Self {
        Self(Self::TEXT_CODE)
    }

    /// Create a packed static string reference op.
    pub(crate) const fn static_text(id: u16) -> Self {
        if id as usize >= Self::MAX_CAP {
            panic!("static op id exceeds packed op capacity");
        }
        Self(Self::STATIC_BASE + id)
    }

    /// Decode this packed op.
    pub(crate) const fn decode(self) -> DecodedTemplateOp {
        if self.0 <= Self::ENTER_MAX_CODE {
            DecodedTemplateOp::Enter {
                skip: self.0 >> 1,
                namespace: self.0 & 1 == 1,
            }
        } else if self.0 == Self::ATTR_CODE {
            DecodedTemplateOp::Attr { namespace: false }
        } else if self.0 == Self::ATTR_CUSTOM_NS_CODE {
            DecodedTemplateOp::Attr { namespace: true }
        } else if self.0 == Self::TEXT_CODE {
            DecodedTemplateOp::Text
        } else {
            DecodedTemplateOp::Static(self.0 - Self::STATIC_BASE)
        }
    }

    /// Get the bits of this packed op.
    pub(crate) const fn bits(self) -> u16 {
        self.0
    }
}

impl std::fmt::Debug for TemplateOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.decode().fmt(f)
    }
}

/// Decoded representation of a packed template operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodedTemplateOp {
    /// Enter an element. `skip` is the number of ops in this element subtree.
    Enter {
        /// Number of ops to skip to move past this element and its children.
        skip: u16,
        /// Whether the reserved namespace string slot contains a namespace.
        namespace: bool,
    },
    /// A static attribute on the current element.
    Attr {
        /// Whether a custom namespace string follows the static attr name/value.
        namespace: bool,
    },
    /// A text node marker. The next op is a [`Self::Static`] string reference.
    Text,
    /// A static string pool reference.
    Static(u16),
}
