use dioxus_core::internal::{FmtSegment, FmtedSegments, HotReloadLiteral};
use dioxus_rsx::*;
use std::cell::Cell;

/// A pool of items we can grab from during hot reloading.
/// We have three different pools we can pull from:
/// - Dynamic text segments (eg: "{class}")
/// - Dynamic nodes (eg: {children})
/// - Dynamic attributes (eg: ..spread )
///
/// As we try to create a new hot reloaded template, we will pull from these pools to create the new template. We mark
/// each item as used the first time we use it in the new template. Once the new template if fully created, we can tally
/// up how many items are unused to determine how well the new template matches the old template.
///
/// The template that matches best will leave the least unused items in the pool.
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct BakedPool<T> {
    pub inner: Vec<BakedItem<T>>,
}

impl<T> BakedPool<T> {
    /// Create a new baked pool from an iterator of items
    fn new(inner: impl IntoIterator<Item = T>) -> Self {
        Self {
            inner: inner.into_iter().map(BakedItem::new).collect(),
        }
    }

    /// Find the first item in the pool that matches the condition and mark it as used
    pub fn position(&self, condition: impl Fn(&T) -> bool) -> Option<usize> {
        for (idx, baked_item) in self.inner.iter().enumerate() {
            if condition(&baked_item.inner) {
                baked_item.used.set(true);
                return Some(idx);
            }
        }
        None
    }

    /// Find the number of unused items in the pool
    fn unused_dynamic_items(&self) -> usize {
        self.inner
            .iter()
            .filter(|baked_item| !baked_item.used.get())
            .count()
    }
}

/// A single item in the baked item pool. We keep track if which items are used for scoring how well two templates match.
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct BakedItem<T> {
    pub inner: T,
    pub used: Cell<bool>,
}

impl<T> BakedItem<T> {
    fn new(inner: T) -> Self {
        Self {
            inner,
            used: Cell::new(false),
        }
    }
}

/// The state of the last full rebuild.
/// This object contains the pool of compiled dynamic segments we can pull from for hot reloading
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct LastBuildState {
    /// The formatted segments that were used in the last build. Eg: "{class}", "{id}"
    ///
    /// We are free to use each of these segments many times in the same build.
    /// We just clone the result (assuming display + debug have no side effects)
    pub dynamic_text_segments: BakedPool<FormattedSegment>,
    /// The dynamic nodes that were used in the last build. Eg: div { {children} }
    ///
    /// We are also free to clone these nodes many times in the same build.
    pub dynamic_nodes: BakedPool<BodyNode>,
    /// The attributes that were used in the last build. Eg: div { class: "{class}" }
    ///
    /// We are also free to clone these nodes many times in the same build.
    pub dynamic_attributes: BakedPool<Attribute>,
    /// The component literal properties we can hot reload from the last build. Eg: Component { class: "{class}" }
    ///
    /// In the new build, we must assign each of these a value even if we no longer use the component.
    /// The type must be the same as the last time we compiled the property
    pub component_properties: Vec<HotLiteral>,
    /// The root indexes of the last build
    pub root_index: DynIdx,
    /// The name of the original template
    pub name: String,
}

impl LastBuildState {
    /// Create a new LastBuildState from the given [`TemplateBody`]
    pub fn new(body: &TemplateBody, name: String) -> Self {
        let dynamic_text_segments = body.dynamic_text_segments.iter().cloned();
        let dynamic_nodes = body.dynamic_nodes().cloned();
        let dynamic_attributes = body.dynamic_attributes().cloned();
        let component_properties = body.literal_component_properties().cloned().collect();
        Self {
            dynamic_text_segments: BakedPool::new(dynamic_text_segments),
            dynamic_nodes: BakedPool::new(dynamic_nodes),
            dynamic_attributes: BakedPool::new(dynamic_attributes),
            component_properties,
            root_index: body.template_idx.clone(),
            name,
        }
    }

    /// Return the number of unused dynamic items in the pool
    pub fn unused_dynamic_items(&self) -> usize {
        self.dynamic_text_segments.unused_dynamic_items()
            + self.dynamic_nodes.unused_dynamic_items()
            + self.dynamic_attributes.unused_dynamic_items()
    }

    /// Hot reload a hot literal
    pub fn hotreload_hot_literal(&self, hot_literal: &HotLiteral) -> Option<HotReloadLiteral> {
        match hot_literal {
            // If the literal is a formatted segment, map the segments to the new formatted segments
            HotLiteral::Fmted(segments) => {
                let new_segments = self.hot_reload_formatted_segments(segments)?;
                Some(HotReloadLiteral::Fmted(new_segments))
            }
            // Otherwise just pass the literal through unchanged
            HotLiteral::Bool(b) => Some(HotReloadLiteral::Bool(b.value())),
            HotLiteral::Float(f) => Some(HotReloadLiteral::Float(f.base10_parse().ok()?)),
            HotLiteral::Int(i) => Some(HotReloadLiteral::Int(i.base10_parse().ok()?)),
        }
    }

    pub fn hot_reload_formatted_segments(
        &self,
        new: &HotReloadFormattedSegment,
    ) -> Option<FmtedSegments> {
        // Go through each dynamic segment and look for a match in the formatted segments pool.
        // If we find a match, we can hot reload the segment otherwise we need to do a full rebuild
        let mut segments = Vec::new();
        for segment in &new.segments {
            match segment {
                // If it is a literal, we can always hot reload it. Just add it to the segments
                Segment::Literal(value) => {
                    segments.push(FmtSegment::Literal {
                        value: Box::leak(value.clone().into_boxed_str()),
                    });
                } // If it is a dynamic segment, we need to check if it exists in the formatted segments pool
                Segment::Formatted(formatted) => {
                    let index = self.dynamic_text_segments.position(|s| s == formatted)?;

                    segments.push(FmtSegment::Dynamic { id: index });
                }
            }
        }

        Some(FmtedSegments::new(segments))
    }
}
