mod anchor;
mod data;
mod ext;
mod op;
mod path;
mod raw;
#[cfg(feature = "serialize")]
mod serialization;
mod storage;

pub use anchor::{TemplateAnchor, TemplateAnchorKind};
pub use data::Template;
pub use ext::TemplateExt;
pub use op::{DecodedTemplateAttrNamespace, DecodedTemplateOp, TemplateOp};
pub use path::{TemplatePath, TemplatePathStep, TemplateSlotPath, TemplateSlotTarget};
pub use raw::TemplateRawOp;
#[cfg(feature = "serialize")]
pub(crate) use serialization::{
    deserialize_leaky, deserialize_option_leaky, deserialize_string_leaky,
    deserialize_strings_leaky,
};
pub(crate) use storage::{
    TemplateStorage, TEMPLATE_STORAGE_DYNAMIC_CAP, TEMPLATE_STORAGE_OPS_CAP,
    TEMPLATE_STORAGE_STRING_CAP,
};
