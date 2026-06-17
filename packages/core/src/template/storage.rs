use dioxus_const_vec::ConstVec;

use super::{
    Template, TemplateAnchor, TemplateAnchorKind, TemplateOp, TemplatePath, TemplateRawOp,
    TemplateSlotPath,
};

/// Maximum packed template storage capacity.
pub(crate) const TEMPLATE_STORAGE_MAX_CAP: usize = TemplateOp::MAX_CAP;

const TEMPLATE_PATH_STACK_CAP: usize = 129;

/// Const storage for a lowered raw template.
///
/// The RSX macro emits a `static TemplateStorage<OPS, STRINGS, DYNAMICS>` from a
/// raw operation tape, then calls [`Self::as_template`] to expose the compact [`Template`] used by
/// the runtime.
#[derive(Clone, Copy)]
pub(crate) struct TemplateStorage<
    const OPS_CAP: usize = TEMPLATE_STORAGE_MAX_CAP,
    const STRING_CAP: usize = TEMPLATE_STORAGE_MAX_CAP,
    const DYNAMIC_CAP: usize = TEMPLATE_STORAGE_MAX_CAP,
> {
    ops: ConstVec<TemplateOp, OPS_CAP>,
    strings: ConstVec<&'static str, STRING_CAP>,
    anchors: ConstVec<TemplateAnchor, DYNAMIC_CAP>,
}

struct RawTemplateLoweringCursor {
    enter_stack: [usize; TEMPLATE_PATH_STACK_CAP],
    element_paths: [TemplatePath; TEMPLATE_PATH_STACK_CAP],
    next_paths: [TemplatePath; TEMPLATE_PATH_STACK_CAP],
    stack_pointer: usize,
}

impl RawTemplateLoweringCursor {
    const fn new() -> Self {
        let mut next_paths = [TemplatePath::empty(); TEMPLATE_PATH_STACK_CAP];
        next_paths[0] = TemplatePath::root(0);
        Self {
            enter_stack: [0; TEMPLATE_PATH_STACK_CAP],
            element_paths: [TemplatePath::empty(); TEMPLATE_PATH_STACK_CAP],
            next_paths,
            stack_pointer: 0,
        }
    }

    const fn open_element(&mut self, enter_index: usize) {
        if self.stack_pointer + 1 >= TEMPLATE_PATH_STACK_CAP {
            panic!("template path stack capacity exceeded");
        }
        let path = self.next_paths[self.stack_pointer];
        self.next_paths[self.stack_pointer] = path.next_sibling();
        self.element_paths[self.stack_pointer] = path;
        self.enter_stack[self.stack_pointer] = enter_index;
        self.next_paths[self.stack_pointer + 1] = path.next_child();
        self.stack_pointer += 1;
    }

    const fn close_element(&mut self) -> usize {
        if self.stack_pointer == 0 {
            panic!("template close op without matching open op");
        }
        self.stack_pointer -= 1;
        self.enter_stack[self.stack_pointer]
    }

    const fn current_element_path(&self) -> TemplatePath {
        if self.stack_pointer == 0 {
            panic!("dynamic attr raw op without an open element");
        }
        self.element_paths[self.stack_pointer - 1]
    }

    const fn current_element_op(&self) -> u16 {
        if self.stack_pointer == 0 {
            panic!("dynamic attr raw op without an open element");
        }
        self.enter_stack[self.stack_pointer - 1] as u16
    }

    const fn node_anchor_op(&self) -> u16 {
        if self.stack_pointer == 0 {
            ROOT_ANCHOR_OP
        } else {
            self.enter_stack[self.stack_pointer - 1] as u16
        }
    }

    const fn next_node_path(&mut self) -> TemplatePath {
        let path = self.next_paths[self.stack_pointer];
        self.next_paths[self.stack_pointer] = path.next_sibling();
        path
    }

    const fn next_slot_path(
        &self,
        raw: &'static [TemplateRawOp],
        index: usize,
    ) -> TemplateSlotPath {
        if self.dynamic_node_has_following_static_at_parent(raw, index) {
            return TemplateSlotPath::before_static(self.next_paths[self.stack_pointer]);
        }

        if self.stack_pointer == 0 {
            TemplateSlotPath::append_children(TemplatePath::empty())
        } else {
            TemplateSlotPath::append_children(self.element_paths[self.stack_pointer - 1])
        }
    }

    const fn finish(&self) {
        if self.stack_pointer != 0 {
            panic!("template raw ops ended with unclosed elements");
        }
    }

    const fn dynamic_node_has_following_static_at_parent(
        &self,
        raw: &'static [TemplateRawOp],
        index: usize,
    ) -> bool {
        let parent_depth = self.stack_pointer;
        let mut depth = parent_depth;
        let mut cursor = index + 1;

        while cursor < raw.len() {
            match raw[cursor] {
                TemplateRawOp::OpenElement { .. } => {
                    if depth == parent_depth {
                        return true;
                    }
                    depth += 1;
                }
                TemplateRawOp::StaticText { .. } => {
                    if depth == parent_depth {
                        return true;
                    }
                }
                TemplateRawOp::CloseElement => {
                    if depth == parent_depth {
                        return false;
                    }
                    depth -= 1;
                }
                TemplateRawOp::StaticAttr { .. }
                | TemplateRawOp::DynamicAttr
                | TemplateRawOp::DynamicNode => {}
            }
            cursor += 1;
        }

        false
    }
}

macro_rules! lower_raw_template {
    ($raw:expr, $builder:ident) => {{
        let mut cursor = RawTemplateLoweringCursor::new();
        let mut index = 0usize;
        while index < $raw.len() {
            match $raw[index] {
                TemplateRawOp::OpenElement { tag, namespace } => {
                    cursor.open_element($builder.ops_len());
                    $builder.push_op(TemplateOp::enter(0, namespace.is_some()));
                    $builder.push_static(tag);
                    if let Some(namespace) = namespace {
                        $builder.push_static(namespace);
                    }
                }
                TemplateRawOp::CloseElement => {
                    let enter_index = cursor.close_element();
                    let namespace = $builder.op_at(enter_index).has_namespace();
                    let skip = $builder.ops_len() - enter_index;
                    if skip > TemplateOp::MAX_CAP {
                        panic!("template op skip exceeds packed op capacity");
                    }
                    $builder.set_op(enter_index, TemplateOp::enter(skip as u16, namespace));
                }
                TemplateRawOp::StaticAttr {
                    name,
                    value,
                    namespace,
                } => {
                    $builder.push_op(TemplateOp::attr(namespace.is_some()));
                    $builder.push_static(name);
                    $builder.push_static(value);
                    if let Some(namespace) = namespace {
                        $builder.push_static(namespace);
                    }
                }
                TemplateRawOp::DynamicAttr => {
                    $builder.push_attr_anchor(
                        cursor.current_element_op(),
                        cursor.current_element_path(),
                    );
                }
                TemplateRawOp::StaticText { value } => {
                    let _ = cursor.next_node_path();
                    $builder.push_op(TemplateOp::text());
                    $builder.push_static(value);
                }
                TemplateRawOp::DynamicNode => {
                    let path = cursor.next_slot_path($raw, index);
                    $builder.push_node_anchor(cursor.node_anchor_op(), path);
                }
            }
            index += 1;
        }
        cursor.finish();
    }};
}

impl<const OPS_CAP: usize, const STRING_CAP: usize, const DYNAMIC_CAP: usize>
    TemplateStorage<OPS_CAP, STRING_CAP, DYNAMIC_CAP>
{
    /// Lower a raw template tape into packed storage in const context.
    pub(crate) const fn build(raw: &'static [TemplateRawOp]) -> Self {
        let mut storage = Self {
            ops: ConstVec::new_with_max_size(),
            strings: ConstVec::new_with_max_size(),
            anchors: ConstVec::new_with_max_size(),
        };

        lower_raw_template!(raw, storage);
        storage.sort_anchors_in_fill_order();
        storage
    }

    /// Return this storage as a compact template.
    pub(crate) const fn as_template(&'static self) -> Template {
        Template::new(
            self.ops.as_slice(),
            self.strings.as_slice(),
            self.anchors.as_slice(),
        )
    }

    const fn push_static(&mut self, value: &'static str) {
        let id = self.strings.len();
        if id >= TemplateOp::MAX_CAP {
            panic!("static op id exceeds packed op capacity");
        }
        self.strings.push(value);
        self.ops.push(TemplateOp::static_text(id as u16));
    }

    const fn ops_len(&self) -> usize {
        self.ops.len()
    }

    const fn op_at(&self, index: usize) -> TemplateOp {
        self.ops.at(index)
    }

    const fn push_op(&mut self, op: TemplateOp) {
        self.ops.push(op);
    }

    const fn set_op(&mut self, index: usize, op: TemplateOp) {
        self.ops.set(index, op);
    }

    const fn push_attr_anchor(&mut self, op: u16, path: TemplatePath) {
        self.push_anchor_bits(op, path.bits(), TemplateAnchorKind::Attr);
    }

    const fn push_node_anchor(&mut self, op: u16, path: TemplateSlotPath) {
        self.push_anchor_bits(op, path.bits(), TemplateAnchorKind::Node);
    }

    const fn push_anchor_bits(&mut self, op: u16, path: u128, kind: TemplateAnchorKind) {
        let len = self.anchors.len();
        if len > 0 {
            let last = self.anchors.at(len - 1);
            if last.same_slot_bits(op, kind, path) {
                self.anchors.set(
                    len - 1,
                    TemplateAnchor {
                        value_count: last.value_count + 1,
                        ..last
                    },
                );
                return;
            }
        }
        let mut i = 0;
        while i < len {
            if self.anchors.at(i).same_slot_bits(op, kind, path) {
                panic!(
                    "dynamic values for a template anchor must be contiguous (attributes must precede children)"
                );
            }
            i += 1;
        }
        let value_start = if len == 0 {
            0
        } else {
            let last = self.anchors.at(len - 1);
            last.value_start + last.value_count
        };
        let anchor = match kind {
            TemplateAnchorKind::Attr => {
                TemplateAnchor::single_attr(op, TemplatePath::from_bits(path), value_start)
            }
            TemplateAnchorKind::Node => {
                TemplateAnchor::single_node(op, TemplateSlotPath::from_bits(path), value_start)
            }
        };
        self.anchors.push(anchor);
    }

    const fn sort_anchors_in_fill_order(&mut self) {
        let len = self.anchors.len();
        let mut index = 0;
        while index < len {
            let mut best = index;
            let mut candidate = index + 1;
            while candidate < len {
                if self
                    .anchors
                    .at(candidate)
                    .should_fill_before(self.anchors.at(best))
                {
                    best = candidate;
                }
                candidate += 1;
            }
            if best != index {
                self.anchors.swap(index, best);
            }
            index += 1;
        }
    }
}

struct RuntimeTemplateBuilder {
    ops: Vec<TemplateOp>,
    strings: Vec<&'static str>,
    anchors: Vec<TemplateAnchor>,
}

impl RuntimeTemplateBuilder {
    fn new() -> Self {
        Self {
            ops: Vec::new(),
            strings: Vec::new(),
            anchors: Vec::new(),
        }
    }

    fn ops_len(&self) -> usize {
        self.ops.len()
    }

    fn op_at(&self, index: usize) -> TemplateOp {
        self.ops[index]
    }

    fn push_op(&mut self, op: TemplateOp) {
        self.ops.push(op);
    }

    fn set_op(&mut self, index: usize, op: TemplateOp) {
        self.ops[index] = op;
    }

    fn push_static(&mut self, value: &'static str) {
        let id = self.strings.len();
        assert!(
            id < TemplateOp::MAX_CAP,
            "static op id exceeds packed op capacity"
        );
        self.strings.push(value);
        self.ops.push(TemplateOp::static_text(id as u16));
    }

    fn push_attr_anchor(&mut self, op: u16, path: TemplatePath) {
        self.push_anchor_bits(op, path.bits(), TemplateAnchorKind::Attr);
    }

    fn push_node_anchor(&mut self, op: u16, path: TemplateSlotPath) {
        self.push_anchor_bits(op, path.bits(), TemplateAnchorKind::Node);
    }

    fn push_anchor_bits(&mut self, op: u16, path: u128, kind: TemplateAnchorKind) {
        if let Some(last) = self.anchors.last_mut() {
            if last.same_slot_bits(op, kind, path) {
                last.value_count += 1;
                return;
            }
        }
        assert!(
            !self
                .anchors
                .iter()
                .any(|a| a.same_slot_bits(op, kind, path)),
            "dynamic values for a template anchor must be contiguous (attributes must precede children)"
        );
        let value_start = self
            .anchors
            .last()
            .map_or(0, |a| a.value_start + a.value_count);
        let anchor = match kind {
            TemplateAnchorKind::Attr => {
                TemplateAnchor::single_attr(op, TemplatePath::from_bits(path), value_start)
            }
            TemplateAnchorKind::Node => {
                TemplateAnchor::single_node(op, TemplateSlotPath::from_bits(path), value_start)
            }
        };
        self.anchors.push(anchor);
    }

    fn into_template(self) -> Template {
        let mut anchors = self.anchors;
        anchors.sort_by(|left, right| {
            if left.should_fill_before(*right) {
                std::cmp::Ordering::Less
            } else if right.should_fill_before(*left) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        Template::new(
            Box::leak(self.ops.into_boxed_slice()),
            Box::leak(self.strings.into_boxed_slice()),
            Box::leak(anchors.into_boxed_slice()),
        )
    }
}

impl Template {
    /// Lower a raw template tape into a leaked runtime template.
    pub(crate) fn from_raw_ops(raw: &'static [TemplateRawOp]) -> Self {
        let mut builder = RuntimeTemplateBuilder::new();
        lower_raw_template!(raw, builder);
        builder.into_template()
    }
}
