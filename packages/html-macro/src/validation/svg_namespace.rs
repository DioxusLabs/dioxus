use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    // list of svg elements
    //  https://developer.mozilla.org/en-US/docs/Web/SVG/Element
    //  a hashmap of `(tag, is_self_closing)`
    static ref SVG_NAMESPACED_TAGS: HashMap<&'static str, bool> = [
        // TODO: can cause conflict with html `a`
        //("a", true),
        ("animate", true),
        ("animateMotion", false),
        ("animateTransform", true),
        ("circle", true),
        ("clipPath",false),
        // TODO: blocked with [issue](https://github.com/chinedufn/percy/issues/106)
        //("color-profile",),
        ("defs", false),
        ("desc", false),
        ("discard", true),
        ("ellipse",true),
        ("feBlend", true),
        ("feColorMatrix", true),
        ("feComponentTransfer", false),
        ("feComposite", true),
        ("feConvolveMatrix", true),
        ("feDiffuseLighting", false),
        ("feDisplacementMap", true),
        ("feDistantLight", true),
        ("feDropShadow", true),
        ("feFlood", true),
        ("feFuncA", true),
        ("feFuncB", true),
        ("feFuncG", true),
        ("feFuncR", true),
        ("feGaussianBlur", true),
        ("feImage", true),
        ("feMerge", false),
        ("feMergeNode", true),
        ("feMorphology", true),
        ("feOffset", true),
        ("fePointLight", true),
        ("feSpecularLighting", false),
        ("feSpotLight", true),
        ("feTile", true),
        ("feTurbulence", true),
        ("filter", false),
        ("foreignObject", false),
        ("g",false),
        ("hatch", false),
        ("hatchpath", true),
        ("image", true),
        ("line", true),
        ("linearGradient", false),
        ("marker", false),
        ("mask", false),
        // TODO: undocumented
        //("mesh",),
        // TODO: undocumented
        //("meshgradient",),
        // TODO: undocumented
        //("meshpatch",),
        // TODO: undocumented
        //("meshrow",),
        ("metadata", false),
        ("mpath", true),
        ("path", true),
        ("pattern", false),
        ("polygon", true),
        ("polyline", true),
        ("radialGradient", false),
        ("rect", true),
        // TODO: can cause conflict with html `script` tag
        //("script", false),
        ("set", true),
        ("solidcolor", true),
        ("stop", true),
        // TODO: can cause conflict with html `style` tag
        //("style", false),
        ("svg", false),
        ("switch", false),
        ("symbol", false),
        ("text", false),
        ("textPath", false),
        // TODO: can cause conflict with html `title` tag
        //("title", false),
        ("tspan", false),
        // TODO: undocumented
        //("unknown",),
        ("use", true),
        ("view", true),
    ]
    .iter()
    .cloned()
    .collect();
}
/// Whether or not this tag is part svg elements
/// ```
/// use html_validation::is_svg_namespace;
///
/// assert_eq!(is_svg_namespace("svg"), true);
///
/// assert_eq!(is_svg_namespace("circle"), true);
///
/// assert_eq!(is_svg_namespace("div"), false);
/// ```
pub fn is_svg_namespace(tag: &str) -> bool {
    SVG_NAMESPACED_TAGS.contains_key(tag)
}

/// Whether or not this svg tag is self closing
pub(crate) fn is_self_closing_svg_tag(tag: &str) -> bool {
    SVG_NAMESPACED_TAGS.get(tag).map(|v| *v).unwrap_or(false)
}
