use super::anchor::ROOT_PARENT_OP_INDEX;
use super::storage::TemplateLoweringCursor;
use super::{TemplatePath, TemplateSlotPath};

/// Storage requirements for lowering a template.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct TemplateStorageStats {
    /// Number of packed template operations.
    pub ops: usize,
    /// Number of static strings.
    pub strings: usize,
    /// Number of dynamic anchors.
    pub anchors: usize,
    /// Number of runtime dynamic node slots.
    pub dynamic_nodes: usize,
    /// Number of runtime dynamic attribute slots.
    pub dynamic_attributes: usize,
    /// Whether lowering overflowed the path stack.
    pub path_overflow: bool,
}

impl TemplateStorageStats {
    fn push_op(&mut self) {
        self.ops += 1;
    }

    fn push_static(&mut self) {
        self.strings += 1;
        self.ops += 1;
    }

    fn push_anchor(
        &mut self,
        anchors: &mut Vec<AnchorStats>,
        parent_op_index: u16,
        path: TemplateSlotPath,
        is_attr: bool,
    ) {
        if is_attr {
            self.dynamic_attributes += 1;
        } else {
            self.dynamic_nodes += 1;
        }

        if let Some(last) = anchors.last_mut() {
            if last.same_anchor(parent_op_index, path) {
                return;
            }
        }

        anchors.push(AnchorStats {
            parent_op_index,
            path,
        });
    }

    fn push_static_anchor(
        &mut self,
        anchors: &mut Vec<AnchorStats>,
        parent_op_index: u16,
        path: TemplateSlotPath,
    ) {
        if let Some(last) = anchors.last_mut() {
            if last.same_anchor(parent_op_index, path) {
                return;
            }
        }

        anchors.push(AnchorStats {
            parent_op_index,
            path,
        });
    }
}

#[derive(Clone, Copy)]
struct AnchorStats {
    parent_op_index: u16,
    path: TemplateSlotPath,
}

impl AnchorStats {
    fn same_anchor(self, parent_op_index: u16, path: TemplateSlotPath) -> bool {
        self.parent_op_index == parent_op_index && self.path == path
    }
}

/// Counts storage requirements for a template without building an operation tape.
pub struct TemplateStatsBuilder {
    stats: TemplateStorageStats,
    cursor: TemplateLoweringCursor,
    anchors: Vec<AnchorStats>,
    namespace_slack: usize,
}

impl Default for TemplateStatsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateStatsBuilder {
    /// Create a new stats builder.
    fn new() -> Self {
        Self {
            stats: TemplateStorageStats::default(),
            cursor: TemplateLoweringCursor::new(),
            anchors: Vec::new(),
            namespace_slack: 0,
        }
    }

    /// Count an element start.
    ///
    /// `namespace` is `Some(true)` when a namespace is known, `Some(false)` when no namespace is
    /// known, and `None` when macro expansion cannot know whether the typed builder will add one.
    pub fn open_element(&mut self, namespace: Option<bool>) {
        self.static_root_anchor();
        let has_namespace = namespace.unwrap_or(false);
        self.cursor.open_element(self.stats.ops, has_namespace);
        self.stats.push_op();
        self.stats.push_static();
        if has_namespace {
            self.stats.push_static();
        } else if namespace.is_none() {
            self.namespace_slack += 1;
        }
    }

    /// Count the end of the current element.
    pub fn close_element(&mut self) {
        let _ = self.cursor.close_element();
    }

    /// Count a static attribute.
    ///
    /// `namespace` follows the same convention as [`Self::open_element`].
    pub fn static_attr(&mut self, namespace: Option<bool>) {
        self.stats.push_op();
        self.stats.push_static();
        self.stats.push_static();
        if namespace == Some(true) {
            self.stats.push_static();
        } else if namespace.is_none() {
            self.namespace_slack += 1;
        }
    }

    /// Count a dynamic attribute slot on the current element.
    pub fn dynamic_attr(&mut self) {
        let frame = self.cursor.current_element_frame();
        if frame.path.is_empty() {
            self.stats.path_overflow = true;
        }
        let path = TemplateSlotPath::static_node(frame.path);
        self.stats
            .push_anchor(&mut self.anchors, frame.enter_index as u16, path, true);
    }

    /// Count a structural root static anchor.
    pub fn static_root_anchor(&mut self) {
        let path = self.cursor.next_paths[self.cursor.stack_pointer];
        if self.cursor.stack_pointer == 0 && !path.is_empty() {
            self.stats.push_static_anchor(
                &mut self.anchors,
                ROOT_PARENT_OP_INDEX,
                TemplateSlotPath::static_node(path),
            );
        }
    }

    /// Count a static text node.
    pub fn static_text(&mut self) {
        self.static_root_anchor();
        let _ = self.cursor.next_node_path();
        self.stats.push_op();
        self.stats.push_static();
    }

    /// Count a dynamic node slot.
    pub fn dynamic_node(&mut self, following_static_at_parent: bool) {
        match self
            .cursor
            .try_next_slot_path_after_dynamic_node(following_static_at_parent)
        {
            Ok(path) => {
                self.stats.push_anchor(
                    &mut self.anchors,
                    self.cursor.node_anchor_parent_op_index(),
                    path,
                    false,
                );
            }
            Err(()) => {
                self.stats.path_overflow = true;
                self.stats.push_anchor(
                    &mut self.anchors,
                    self.cursor.node_anchor_parent_op_index(),
                    TemplateSlotPath::last_static_node(TemplatePath::empty()),
                    false,
                );
            }
        }
    }

    /// Finish counting and return the storage requirements.
    pub fn finish(mut self) -> TemplateStorageStats {
        self.cursor.finish();
        self.stats.anchors = self.anchors.len();

        // Each unknown namespace may emit one extra static string and one extra packed op.
        self.stats.ops += self.namespace_slack;
        self.stats.strings += self.namespace_slack;

        self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::TemplateStorage;
    use crate::{Template, TemplateRawTree};

    fn template_from_tree(tree: &'static TemplateRawTree) -> Template {
        TemplateStorage::<64, 64, 16>::build_from_tree(tree).into_leaked_template()
    }

    #[test]
    fn stats_builder_can_overestimate_unknown_namespaces() {
        let mut stats = TemplateStatsBuilder::new();
        stats.open_element(None);
        stats.static_attr(None);
        stats.close_element();
        let stats = stats.finish();

        static ATTR: TemplateRawTree = TemplateRawTree::StaticAttr {
            name: "class",
            value: "name",
            namespace: None,
        };
        static TREE: TemplateRawTree = TemplateRawTree::Element {
            tag: "div",
            namespace: None,
            attrs: &ATTR,
            children: &TemplateRawTree::Empty,
        };
        let template = template_from_tree(&TREE);

        assert!(stats.ops >= template.decoded_ops().len());
        assert!(stats.strings >= template.strings().len());
        assert_eq!(stats.anchors, template.anchors().len());
        let dynamic_node_count = template
            .anchors()
            .iter()
            .map(|anchor| anchor.nodes().end)
            .max()
            .unwrap_or_default();
        let dynamic_attribute_count = template
            .anchors()
            .iter()
            .map(|anchor| anchor.attributes().end)
            .max()
            .unwrap_or_default();
        assert_eq!(stats.dynamic_nodes, dynamic_node_count);
        assert_eq!(stats.dynamic_attributes, dynamic_attribute_count);
    }
}
