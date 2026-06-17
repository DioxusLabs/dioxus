mod anchor;
mod data;
mod ext;
mod op;
mod path;
mod raw;
#[cfg(feature = "serialize")]
mod serialization;
mod storage;

pub use anchor::{ROOT_ANCHOR_OP, TemplateAnchor, TemplateAnchorKind};
pub use data::Template;
pub use ext::TemplateExt;
pub use op::{DecodedTemplateAttrNamespace, DecodedTemplateOp, TemplateOp};
pub use path::{TemplatePath, TemplatePathIter, TemplatePathStep, TemplateSlotPath, TemplateSlotTarget};
pub use raw::{TemplateRawAttrNamespace, TemplateRawOp};
#[cfg(feature = "serialize")]
pub(crate) use serialization::{
    deserialize_leaky, deserialize_option_leaky, deserialize_string_leaky, deserialize_strings_leaky,
};
pub(crate) use storage::{TEMPLATE_STORAGE_MAX_CAP, TemplateStorage};
