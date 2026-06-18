pub use dioxus_core_template::{
    DecodedTemplateOp, Template, TemplateAnchor, TemplateOp, TemplatePath, TemplateRawOp,
    TemplateRawTree, TemplateSlotPath, TemplateSlotTarget,
};

#[cfg(feature = "serialize")]
pub(crate) use dioxus_core_template::{
    deserialize_leaky, deserialize_option_leaky, deserialize_string_leaky,
    deserialize_strings_leaky,
};

pub(crate) use dioxus_core_template::TemplateStorage;
#[cfg(debug_assertions)]
pub(crate) use dioxus_core_template::{
    TEMPLATE_STORAGE_DYNAMIC_CAP, TEMPLATE_STORAGE_OPS_CAP, TEMPLATE_STORAGE_STRING_CAP,
};
