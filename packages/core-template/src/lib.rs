mod anchor;
mod data;
mod op;
mod path;
mod raw;
#[cfg(feature = "serialize")]
mod serialization;
mod storage;

pub use anchor::TemplateAnchor;
pub use data::Template;
pub use op::{DecodedTemplateAttrNamespace, DecodedTemplateOp, TemplateOp};
pub use path::{TemplatePath, TemplateSlotPath, TemplateSlotTarget};
pub use raw::TemplateRawTree;
#[cfg(feature = "serialize")]
#[doc(hidden)]
pub use serialization::{
    deserialize_leaky, deserialize_option_leaky, deserialize_string_leaky,
    deserialize_strings_leaky,
};
pub use storage::TemplateStorageStats;
#[doc(hidden)]
pub use storage::{
    RuntimeTemplateBuilder, TEMPLATE_STORAGE_DYNAMIC_CAP, TEMPLATE_STORAGE_MAX_CAP,
    TEMPLATE_STORAGE_OPS_CAP, TEMPLATE_STORAGE_STRING_CAP, TemplateStatsBuilder, TemplateStorage,
};
