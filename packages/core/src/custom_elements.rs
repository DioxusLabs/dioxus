use std::marker::PhantomData;

/// A description of an attribute for use by the rsx
pub struct AttributeDescription {
    pub name: &'static str,
    pub namespace: Option<&'static str>,
    pub volatile: bool,
}
