use dioxus_core_template::{TemplateLoweringCursor, TemplateSlotPath};

/// Storage requirements for lowering a template.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TemplateStorageStats {
    /// Number of packed template operations.
    pub(crate) ops: usize,
    /// Number of static strings.
    pub(crate) strings: usize,
    /// Number of dynamic anchors.
    pub(crate) anchors: usize,
    /// Number of runtime dynamic node slots.
    pub(crate) dynamic_nodes: usize,
    /// Number of runtime dynamic attribute slots.
    pub(crate) dynamic_attributes: usize,
    /// Whether lowering overflowed the path stack.
    pub(crate) path_overflow: bool,
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
pub(crate) struct TemplateStatsBuilder {
    cursor: TemplateLoweringCursor,
    anchors: Vec<AnchorStats>,
    ops: usize,
    strings: usize,
    dynamic_nodes: usize,
    dynamic_attributes: usize,
    path_overflow: bool,
    namespace_slack: usize,
}

impl Default for TemplateStatsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateStatsBuilder {
    fn new() -> Self {
        Self {
            cursor: TemplateLoweringCursor::new(),
            anchors: Vec::new(),
            ops: 0,
            strings: 0,
            dynamic_nodes: 0,
            dynamic_attributes: 0,
            path_overflow: false,
            namespace_slack: 0,
        }
    }

    pub(crate) fn open_element(&mut self, namespace: Option<bool>) {
        self.static_root_anchor();
        let has_namespace = namespace.unwrap_or(false);
        self.cursor.open_element(self.ops, has_namespace);
        self.push_op();
        self.push_static();
        if has_namespace {
            self.push_static();
        } else if namespace.is_none() {
            self.namespace_slack += 1;
        }
    }

    pub(crate) fn close_element(&mut self) {
        let _ = self.cursor.close_element();
    }

    pub(crate) fn static_attr(&mut self, namespace: Option<bool>) {
        self.push_op();
        self.push_static();
        self.push_static();
        if namespace == Some(true) {
            self.push_static();
        } else if namespace.is_none() {
            self.namespace_slack += 1;
        }
    }

    pub(crate) fn dynamic_attr(&mut self) {
        let (parent_op_index, path, overflow) = self.cursor.dynamic_attr_anchor();
        self.path_overflow |= overflow;
        self.push_anchor(parent_op_index, path, true);
    }

    pub(crate) fn static_root_anchor(&mut self) {
        if let Some((parent_op_index, path)) = self.cursor.static_root_anchor() {
            self.push_static_anchor(parent_op_index, path);
        }
    }

    pub(crate) fn static_text(&mut self) {
        self.static_root_anchor();
        let _ = self.cursor.next_node_path();
        self.push_op();
        self.push_static();
    }

    pub(crate) fn dynamic_node(&mut self, following_static_at_parent: bool) {
        let (parent_op_index, path, overflow) =
            self.cursor.dynamic_node_anchor(following_static_at_parent);
        self.path_overflow |= overflow;
        self.push_anchor(parent_op_index, path, false);
    }

    pub(crate) fn finish(mut self) -> TemplateStorageStats {
        self.cursor.finish();

        // Each unknown namespace may emit one extra static string and one extra packed op.
        self.ops += self.namespace_slack;
        self.strings += self.namespace_slack;

        TemplateStorageStats {
            ops: self.ops,
            strings: self.strings,
            anchors: self.anchors.len(),
            dynamic_nodes: self.dynamic_nodes,
            dynamic_attributes: self.dynamic_attributes,
            path_overflow: self.path_overflow,
        }
    }

    fn push_op(&mut self) {
        self.ops += 1;
    }

    fn push_static(&mut self) {
        self.strings += 1;
        self.ops += 1;
    }

    fn push_anchor(&mut self, parent_op_index: u16, path: TemplateSlotPath, is_attr: bool) {
        if is_attr {
            self.dynamic_attributes += 1;
        } else {
            self.dynamic_nodes += 1;
        }

        if let Some(last) = self.anchors.last_mut()
            && last.same_anchor(parent_op_index, path)
        {
            return;
        }

        self.anchors.push(AnchorStats {
            parent_op_index,
            path,
        });
    }

    fn push_static_anchor(&mut self, parent_op_index: u16, path: TemplateSlotPath) {
        if let Some(last) = self.anchors.last_mut()
            && last.same_anchor(parent_op_index, path)
        {
            return;
        }

        self.anchors.push(AnchorStats {
            parent_op_index,
            path,
        });
    }
}
