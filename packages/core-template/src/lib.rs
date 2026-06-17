mod anchor;
mod data;
mod ext;
mod op;
mod path;
mod raw;
#[cfg(feature = "serialize")]
mod serialization;
mod storage;

pub use anchor::TemplateAnchor;
pub use data::Template;
pub use ext::TemplateExt;
pub use op::{DecodedTemplateAttrNamespace, DecodedTemplateOp, TemplateOp};
pub use path::{TemplatePath, TemplatePathStep, TemplateSlotPath, TemplateSlotTarget};
pub use raw::{TemplateRawOp, TemplateRawTree};
#[cfg(feature = "serialize")]
#[doc(hidden)]
pub use serialization::{
    deserialize_leaky, deserialize_option_leaky, deserialize_string_leaky,
    deserialize_strings_leaky,
};
pub use storage::{
    TEMPLATE_STORAGE_DYNAMIC_CAP, TEMPLATE_STORAGE_MAX_CAP, TEMPLATE_STORAGE_OPS_CAP,
    TEMPLATE_STORAGE_STRING_CAP, TemplateStorage, TemplateStorageStats,
};

/// Default raw template operation storage capacity.
pub const TEMPLATE_RAW_OPS_CAP: usize = 256;
