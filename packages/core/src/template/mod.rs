pub use dioxus_core_template::{
    DecodedTemplateAttrNamespace, DecodedTemplateOp, Template, TemplateAnchor, TemplateOp,
    TemplatePath, TemplateRawTree, TemplateSlotPath, TemplateSlotTarget, TemplateStorageStats,
};

#[cfg(feature = "serialize")]
pub(crate) use dioxus_core_template::{
    deserialize_leaky, deserialize_option_leaky, deserialize_string_leaky,
    deserialize_strings_leaky,
};

#[doc(hidden)]
pub use dioxus_core_template::RuntimeTemplateBuilder;
pub(crate) use dioxus_core_template::TemplateStorage;
#[cfg(debug_assertions)]
pub(crate) use dioxus_core_template::{
    TEMPLATE_STORAGE_DYNAMIC_CAP, TEMPLATE_STORAGE_OPS_CAP, TEMPLATE_STORAGE_STRING_CAP,
};
