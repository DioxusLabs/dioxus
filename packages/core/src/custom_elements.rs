use std::marker::PhantomData;

/// The raw definition of an element
///
/// This should be compiled away
pub struct CustomElement<N, E = ()> {
    pub tag: &'static str,
    pub namespace: Option<&'static str>,
    pub volatile: bool,
    _t: PhantomData<(N, E)>,
}

/// A description of an attribute for use by the rsx
pub struct AttributeDescription {
    pub name: &'static str,
    pub namespace: Option<&'static str>,
    pub is_volatile: bool,
}
