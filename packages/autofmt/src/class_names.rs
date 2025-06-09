//! Implements autosorting of Tailwind classes according to their opinionated class name sorting:
//! https://tailwindcss.com/blog/automatic-class-sorting-with-prettier#how-classes-are-sorted
//!
//! This methodology should match the sorting of their Prettier plugin.

use std::cmp::Ordering;

use dioxus_rsx::{AttributeValue, BodyNode, CallBody, HotReloadFormattedSegment};
use itertools::Itertools;
use lightningcss::{
    printer::PrinterOptions,
    rules::CssRule,
    stylesheet::{ParserOptions, StyleSheet},
    traits::ToCss,
};
use syn::LitStr;

#[derive(Debug, Clone)]
pub struct TailwindSorter {
    class_order: Vec<String>,
}

impl TailwindSorter {
    /// Create a new instance of the sorter class.
    /// Needs to receive an instance of compiled Tailwind CSS,
    /// so it can parse the available classes.
    pub fn new(tailwind_css: &str) -> Option<Self> {
        let css_rules = StyleSheet::parse(tailwind_css, ParserOptions::default())
            .ok()?
            .rules
            .0;

        // Get each layer's class names and sort each of them according to Tailwind's priorities.
        let base_classes = find_layer(&css_rules, "base")
            .map(|layer| get_class_names(layer))
            .unwrap_or_default();
        let components_classes = find_layer(&css_rules, "components")
            .map(|layer| get_class_names(layer))
            .unwrap_or_default();
        let utilities_classes = find_layer(&css_rules, "utilities")
            .map(|layer| get_class_names(layer))
            .unwrap_or_default();

        // Now we merge all the layers together, along with non-Tailwind CSS classes at the front.
        let mut class_order = Vec::with_capacity(
            base_classes.len() + components_classes.len() + utilities_classes.len(),
        );
        class_order.extend(base_classes);
        class_order.extend(components_classes);
        class_order.extend(utilities_classes);
        sort_tailwind_classes(&mut class_order);

        Some(Self { class_order })
    }

    /// Returns a sorted list of class names given an input class string.
    pub fn sort_class_names(&self, classes: &str) -> String {
        // First sort alphabetically, for any classes that are not part of Tailwind.
        // We will give them all an equivalent priority of -1 and stable-sort in phase 2
        // so that they remain alphabetized as the front of the list.
        let mut class_list = classes.split_ascii_whitespace().collect::<Vec<_>>();
        class_list.sort_unstable();

        // Assign a priority value to each class that is found, and use this to sort them.
        class_list.sort_by_key(|class| {
            self.class_order
                .iter()
                .position(|tailwind_class| tailwind_class == class)
                .map(|idx| idx as i32)
                .unwrap_or(-1)
        });
        class_list.join(" ")
    }
}

fn find_layer<'a, 'b>(ruleset: &'a [CssRule<'b>], layer_name: &str) -> Option<&'a [CssRule<'b>]> {
    ruleset.iter().find_map(|rule| match rule {
        lightningcss::rules::CssRule::LayerBlock(layer_block_rule)
            if layer_block_rule
                .name
                .as_ref()
                .and_then(|name| name.0.first())
                .map(|name| name.as_ref())
                == Some(layer_name) =>
        {
            Some(layer_block_rule.rules.0.as_slice())
        }
        _ => None,
    })
}

fn get_class_names(ruleset: &[CssRule]) -> Vec<String> {
    let mut raw_items = Vec::new();
    for rule in ruleset.iter() {
        match rule {
            CssRule::Style(style_rule) => {
                if let Some(selector) = style_rule.selectors.0.first() {
                    raw_items.push(
                        selector
                            .to_css_string(PrinterOptions::default())
                            .unwrap_or_default(),
                    );
                }
            }
            CssRule::Media(media_rule) => {
                for rule in &media_rule.rules.0 {
                    if let CssRule::Style(style_rule) = rule {
                        if let Some(selector) = style_rule.selectors.0.first() {
                            let selector = selector
                                .to_css_string(PrinterOptions::default())
                                .unwrap_or_default();
                            if selector
                                .split(':')
                                .filter(|part| !part.starts_with("."))
                                .count()
                                > 1
                            {
                                raw_items.push(selector.rsplit_once(':').unwrap().0.to_string());
                            } else {
                                raw_items.push(selector);
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }

    raw_items
        .into_iter()
        .filter_map(|selector| {
            if selector.starts_with('.') && selector.len() > 1 {
                Some(selector[1..].replace("\\:", ":"))
            } else {
                None
            }
        })
        .unique()
        .collect::<Vec<_>>()
}

fn sort_tailwind_classes(classes: &mut [String]) {
    // We want classes with modifiers to come last, and alphabetized by their prefixes.
    // Must be a stable sort, to keep the suffixes in the correct order.
    classes.sort_by(|class_a, class_b| {
        match (class_a.rsplit_once(':'), class_b.rsplit_once(':')) {
            (None, None) => Ordering::Equal,
            (Some((prefix_a, _)), Some((prefix_b, _))) => prefix_a.cmp(prefix_b),
            (Some((_prefix_a, _)), None) => Ordering::Greater,
            (None, Some((_prefix_b, _))) => Ordering::Less,
        }
    });

    // However, we want responsive classes sorted by size, and grouped together.
    // This also must be a stable sort.
    classes.sort_by(|class_a, class_b| {
        const RESPONSIVE_SIZES: &[&str] = &["sm", "md", "lg", "xl", "2xl"];

        match (class_a.split_once(':'), class_b.split_once(':')) {
            (Some((prefix_a, _)), Some((prefix_b, _))) => {
                let prefix_a_idx = RESPONSIVE_SIZES.iter().position(|size| size == &prefix_a);
                let prefix_b_idx = RESPONSIVE_SIZES.iter().position(|size| size == &prefix_b);

                if let Some(prefix_a_idx) = prefix_a_idx {
                    if let Some(prefix_b_idx) = prefix_b_idx {
                        // Both have a responsive sizing, so sort them
                        prefix_a_idx.cmp(&prefix_b_idx)
                    } else {
                        // Only prefix_a has a responsive sizing, so put it last
                        Ordering::Greater
                    }
                } else if prefix_b_idx.is_some() {
                    // Only prefix_b has a responsive sizing, so put it last
                    Ordering::Less
                } else {
                    // Neither has a responsive sizing, so maintain current sorting
                    Ordering::Equal
                }
            }
            _ => Ordering::Equal,
        }
    });

    // For the rest of the sorting, it seems safe to assume that the
    // sorting of classes in the input tailwind.css file is correct.
}

pub(crate) fn format_class_attrs(body: &mut CallBody, sorter: &TailwindSorter) {
    for item in &mut body.body.roots {
        format_elem_class_attrs(item, sorter);
    }
}

fn format_elem_class_attrs(item: &mut BodyNode, sorter: &TailwindSorter) {
    match item {
        BodyNode::Element(element) => {
            let class_attr = element
                .raw_attributes
                .iter_mut()
                .find(|attr| match &attr.name {
                    dioxus_rsx::AttributeName::BuiltIn(ident) => *ident == "class",
                    _ => false,
                });
            if let Some(class_attr) = class_attr {
                format_class_attr_value(&mut class_attr.value, sorter);
            }

            for child in &mut element.children {
                format_elem_class_attrs(child, sorter);
            }
        }
        BodyNode::Component(component) => {
            let class_attr = component.fields.iter_mut().find(|attr| match &attr.name {
                dioxus_rsx::AttributeName::BuiltIn(ident) => *ident == "class",
                _ => false,
            });
            if let Some(class_attr) = class_attr {
                format_class_attr_value(&mut class_attr.value, sorter);
            }

            for child in &mut component.children.roots.iter_mut() {
                format_elem_class_attrs(child, sorter);
            }
        }
        BodyNode::ForLoop(for_loop) => {
            for child in &mut for_loop.body.roots.iter_mut() {
                format_elem_class_attrs(child, sorter);
            }
        }
        BodyNode::IfChain(if_chain) => {
            for child in &mut if_chain.then_branch.roots.iter_mut() {
                format_elem_class_attrs(child, sorter);
            }
            if let Some(else_branch) = if_chain.else_branch.as_mut() {
                for child in else_branch.roots.iter_mut() {
                    format_elem_class_attrs(child, sorter);
                }
            }
        }
        _ => (),
    }
}

fn format_class_attr_value(value: &mut AttributeValue, sorter: &TailwindSorter) {
    match value {
        dioxus_rsx::AttributeValue::AttrLiteral(dioxus_rsx::HotLiteral::Fmted(lit)) => {
            format_class_hot_literal(lit, sorter);
        }
        dioxus_rsx::AttributeValue::IfExpr(if_attribute_value) => {
            // We can format string literals inside if exprs as well
            format_class_attr_value(&mut if_attribute_value.then_value, sorter);
            if let Some(else_value) = if_attribute_value.else_value.as_mut() {
                format_class_attr_value(else_value, sorter);
            }
        }
        _ => (),
    }
}

fn format_class_hot_literal(lit: &mut HotReloadFormattedSegment, sorter: &TailwindSorter) {
    // We can really only consider this safe to format if it's a string literal,
    // which will have a singular "Literal" segment.
    if lit.formatted_input.segments.len() != 1 {
        return;
    }

    if let dioxus_rsx::Segment::Literal(class_str) = &mut lit.formatted_input.segments[0] {
        *class_str = sorter.sort_class_names(class_str);
        lit.formatted_input.source = LitStr::new(class_str, lit.formatted_input.source.span());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CSS: &str = include_str!("../tests/test_css.css");

    #[test]
    fn sorts_classes_correctly() {
        // This list of examples is pulled from Tailwind's documentation
        let before = "text-white px-4 sm:px-8 py-2 sm:py-3 bg-sky-700 hover:bg-sky-800";
        let after = "bg-sky-700 px-4 py-2 text-white hover:bg-sky-800 sm:px-8 sm:py-3";
        assert_eq!(
            &TailwindSorter::new(TEST_CSS)
                .unwrap()
                .sort_class_names(before),
            after
        );
    }

    #[test]
    fn sorts_classes_by_layer() {
        // Any classes in the base layer will be sorted first,
        // followed by classes in the components layer,
        // and then finally classes in the utilities layer.
        let before = "container mx-auto px-6";
        let after = "container mx-auto px-6";
        assert_eq!(
            &TailwindSorter::new(TEST_CSS)
                .unwrap()
                .sort_class_names(before),
            after
        );
    }

    #[test]
    fn sorts_classes_by_priority() {
        // Utilities themselves are sorted in the same order we sort them in the CSS as well,
        // which means that any classes that override other classes
        // always appear later in the class list:
        let before = "pt-2 p-4";
        let after = "p-4 pt-2";
        assert_eq!(
            &TailwindSorter::new(TEST_CSS)
                .unwrap()
                .sort_class_names(before),
            after
        );
    }

    #[test]
    fn sorts_classes_by_impact() {
        // The actual order of the different utilities is loosely based on the box model,
        // and tries to put high impact classes that affect the layout at the beginning
        // and decorative classes at the end, while also trying to keep related utilities together:
        let before = "text-gray-700 shadow-md p-3 border-gray-300 ml-4 h-24 flex border-2";
        let after = "ml-4 flex h-24 border-2 border-gray-300 p-3 text-gray-700 shadow-md";
        assert_eq!(
            &TailwindSorter::new(TEST_CSS)
                .unwrap()
                .sort_class_names(before),
            after
        );
    }

    #[test]
    fn sorts_classes_by_modifiers() {
        // Modifiers like hover: and focus: are grouped together and sorted after any plain utilities:
        let before = "hover:opacity-75 opacity-50 hover:scale-150 scale-125";
        let after = "scale-125 opacity-50 hover:scale-150 hover:opacity-75";
        assert_eq!(
            &TailwindSorter::new(TEST_CSS)
                .unwrap()
                .sort_class_names(before),
            after
        );
    }

    #[test]
    fn sorts_classes_by_responsive_size() {
        // Responsive modifiers like md: and lg: are grouped together at the end
        // in the same order they're configured in your theme
        // — which is smallest to largest by default:
        let before = "lg:grid-cols-4 grid sm:grid-cols-3 grid-cols-2";
        let after = "grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4";
        assert_eq!(
            &TailwindSorter::new(TEST_CSS)
                .unwrap()
                .sort_class_names(before),
            after
        );
    }

    #[test]
    fn sorts_custom_classes_first() {
        // Any custom classes that don't come from Tailwind plugins
        // (like classes for targeting a third-party library) are always sorted to the front,
        // so it's easy to see when an element is using them:
        let before = "p-3 shadow-xl select2-dropdown";
        let after = "select2-dropdown p-3 shadow-xl";
        assert_eq!(
            &TailwindSorter::new(TEST_CSS)
                .unwrap()
                .sort_class_names(before),
            after
        );
    }
}
