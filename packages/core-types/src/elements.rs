use serde::{Serialize, Deserialize}

/// the attribute specification, to be used with `manganis::register_element!`
#[derive(Serialize, Deserialize)]
pub struct AttributeSpecification {
    /// the name of the attribute.
    /// This is not the rust name: if this attribute has name `type`,
    /// it will be used as `rsx!{ element { r#type = ... } }`
    name: Option<&'static str>,
    namespace: Option<&'static str>,
}


/// the element specification, to be used with `manganis::register_element!`
/// Example:
/// ```rust
/// manganis::register_elemenent!(
///     ElementSpecification {
///         name: "my_div",
///         namespace: None,
///         attribute_group: Some("html_element"),
///         special_attributes: &[
///             AttributeSpecification {name: "fancy_color", namespace: None}
///         ]
///     }
/// )
/// ```
/// make sur that the name of the attribute group you use is defined in a library you included in
/// your app.
pub struct ElementSpecification {
    /// the name of the element.
    /// This is not the rust name: if this attribute has name `await`,
    /// it will be used as `rsx!{ r#await { ... } }`
    name: &'static,
    namespace: Option<&'static str>,
    /// the optional attribute group, like `html_element` or `svg_element`.
    attribute_group: Option<&'static str>,
    /// the attributes that are not included in the attribute group.
    special_attributes: &'static [AttributeSpecification],
}

/// the attribute group specification, to be used with `manganis::register_element!`
/// Example:
/// ```rust
/// manganis::register_attribute_group!(
///     AttributeGroupSpecification {
///         name: "my_div",
///         attributes: &[
///             AttributeSpecification {name: "fancy_color", namespace: None}
///         ]
///     }
/// )
/// ```
/// make sur that the name of the attribute group you use is defined in a library you included in
/// your app.
pub struct AttributeGroupSpecification {
    /// the identifier of this attribute group
    name: &'static str,
    /// 
    attributes: &'static [AttributeSpecification],
}
