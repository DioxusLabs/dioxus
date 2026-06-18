mod anchor;
mod data;
mod op;
mod path;
mod raw;
#[cfg(feature = "serialize")]
mod serialization;
mod storage;

#[doc(hidden)]
pub use anchor::TemplateAnchor;
pub use data::Template;
#[doc(hidden)]
pub use op::{DecodedTemplateAttrNamespace, DecodedTemplateOp, TemplateOp};
#[doc(hidden)]
pub use path::{TemplatePath, TemplateSlotPath, TemplateSlotTarget};
#[doc(hidden)]
pub use raw::{TemplateRawOp, TemplateRawTree};
#[cfg(feature = "serialize")]
#[doc(hidden)]
pub use serialization::{
    deserialize_leaky, deserialize_option_leaky, deserialize_string_leaky,
    deserialize_strings_leaky,
};
#[doc(hidden)]
pub use storage::{
    TEMPLATE_STORAGE_DYNAMIC_CAP, TEMPLATE_STORAGE_MAX_CAP, TEMPLATE_STORAGE_OPS_CAP,
    TEMPLATE_STORAGE_STRING_CAP, TemplateStorage, TemplateStorageStats,
};

/// Default raw template operation storage capacity.
#[doc(hidden)]
pub const TEMPLATE_RAW_OPS_CAP: usize = 256;
