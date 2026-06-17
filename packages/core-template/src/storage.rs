use dioxus_const_vec::ConstVec;

use super::anchor::ROOT_ANCHOR_OP;
use super::{
    Template, TemplateAnchor, TemplateOp, TemplatePath, TemplateRawOp, TemplateRawTree,
    TemplateSlotPath,
};
use crate::TEMPLATE_RAW_OPS_CAP;

/// Maximum packed template storage capacity.
pub const TEMPLATE_STORAGE_MAX_CAP: usize = TemplateOp::MAX_CAP;

/// Default packed template operation storage capacity.
pub const TEMPLATE_STORAGE_OPS_CAP: usize = 512;

/// Default static string storage capacity.
pub const TEMPLATE_STORAGE_STRING_CAP: usize = 256;

/// Default dynamic anchor storage capacity.
pub const TEMPLATE_STORAGE_DYNAMIC_CAP: usize = 32;

const TEMPLATE_PATH_STACK_CAP: usize = 32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TemplateStorageStats {
    pub raw_ops: usize,
    pub ops: usize,
    pub strings: usize,
    pub anchors: usize,
    pub dynamic_values: usize,
    pub path_overflow: bool,
}

#[derive(Clone, Copy)]
struct AnchorStats {
    op: u16,
    path: u128,
    value_count: usize,
}

/// Const storage for a lowered raw template.
///
/// The RSX macro emits a `static TemplateStorage<OPS, STRINGS, DYNAMICS>` from a
/// raw operation tape, then calls [`Self::as_template`] to expose the compact [`Template`] used by
/// the runtime.
#[derive(Clone, Copy)]
pub struct TemplateStorage<
    const OPS_CAP: usize = TEMPLATE_STORAGE_OPS_CAP,
    const STRING_CAP: usize = TEMPLATE_STORAGE_STRING_CAP,
    const DYNAMIC_CAP: usize = TEMPLATE_STORAGE_DYNAMIC_CAP,
> {
    ops: ConstVec<TemplateOp, OPS_CAP>,
    strings: ConstVec<&'static str, STRING_CAP>,
    anchors: ConstVec<TemplateAnchor, DYNAMIC_CAP>,
}

#[derive(Clone, Copy)]
struct TemplateElementFrame {
    enter_index: usize,
    namespace: bool,
    path: TemplatePath,
}

struct TemplateLoweringCursor {
    enter_stack: [TemplateElementFrame; TEMPLATE_PATH_STACK_CAP],
    next_paths: [TemplatePath; TEMPLATE_PATH_STACK_CAP],
    stack_pointer: usize,
}

impl TemplateLoweringCursor {
    const fn new() -> Self {
        let mut next_paths = [TemplatePath::empty(); TEMPLATE_PATH_STACK_CAP];
        next_paths[0] = TemplatePath::root(0);
        Self {
            enter_stack: [TemplateElementFrame {
                enter_index: 0,
                namespace: false,
                path: TemplatePath::empty(),
            }; TEMPLATE_PATH_STACK_CAP],
            next_paths,
            stack_pointer: 0,
        }
    }

    const fn open_element(&mut self, enter_index: usize, namespace: bool) {
        if self.stack_pointer + 1 >= TEMPLATE_PATH_STACK_CAP {
            panic!("template path stack capacity exceeded");
        }
        let path = self.next_paths[self.stack_pointer];
        self.next_paths[self.stack_pointer] = path.next_sibling();
        self.enter_stack[self.stack_pointer] = TemplateElementFrame {
            enter_index,
            namespace,
            path,
        };
        self.next_paths[self.stack_pointer + 1] = path.next_child();
        self.stack_pointer += 1;
    }

    const fn close_element(&mut self) -> (usize, bool) {
        if self.stack_pointer == 0 {
            panic!("template close op without matching open op");
        }
        self.stack_pointer -= 1;
        let frame = self.enter_stack[self.stack_pointer];
        (frame.enter_index, frame.namespace)
    }

    const fn current_element_path(&self) -> TemplatePath {
        if self.stack_pointer == 0 {
            panic!("dynamic attr raw op without an open element");
        }
        self.current_element_frame().path
    }

    const fn current_element_op(&self) -> u16 {
        self.current_element_frame().enter_index as u16
    }

    const fn node_anchor_op(&self) -> u16 {
        if self.stack_pointer == 0 {
            ROOT_ANCHOR_OP
        } else {
            self.current_element_frame().enter_index as u16
        }
    }

    const fn current_element_frame(&self) -> TemplateElementFrame {
        if self.stack_pointer == 0 {
            panic!("template cursor is not inside an element");
        }
        let frame = self.enter_stack[self.stack_pointer - 1];
        if frame.enter_index > TemplateOp::MAX_CAP {
            panic!("template enter op exceeds packed op capacity");
        }
        frame
    }

    const fn next_node_path(&mut self) -> TemplatePath {
        let path = self.next_paths[self.stack_pointer];
        self.next_paths[self.stack_pointer] = path.next_sibling();
        path
    }

    const fn next_slot_path(&self, raw: &[TemplateRawOp], index: usize) -> TemplateSlotPath {
        self.next_slot_path_after_dynamic_node(
            self.dynamic_node_has_following_static_at_parent(raw, index),
        )
    }

    const fn next_slot_path_after_dynamic_node(
        &self,
        has_following_static_at_parent: bool,
    ) -> TemplateSlotPath {
        if has_following_static_at_parent {
            return TemplateSlotPath::before_static(self.next_paths[self.stack_pointer]);
        }

        if self.stack_pointer == 0 {
            TemplateSlotPath::append_children(TemplatePath::empty())
        } else {
            TemplateSlotPath::append_children(self.current_element_path())
        }
    }

    fn try_next_slot_path(
        &self,
        raw: &[TemplateRawOp],
        index: usize,
    ) -> Result<TemplateSlotPath, ()> {
        self.try_next_slot_path_after_dynamic_node(
            self.dynamic_node_has_following_static_at_parent(raw, index),
        )
    }

    fn try_next_slot_path_after_dynamic_node(
        &self,
        has_following_static_at_parent: bool,
    ) -> Result<TemplateSlotPath, ()> {
        if has_following_static_at_parent {
            let path = self.next_paths[self.stack_pointer];
            if path.is_empty() {
                return Err(());
            }
            return Ok(TemplateSlotPath::before_static(path));
        }

        if self.stack_pointer == 0 {
            Ok(TemplateSlotPath::append_children(TemplatePath::empty()))
        } else {
            let path = self.current_element_path();
            if path.is_empty() {
                return Err(());
            }
            Ok(TemplateSlotPath::append_children(path))
        }
    }

    const fn finish(&self) {
        if self.stack_pointer != 0 {
            panic!("template raw ops ended with unclosed elements");
        }
    }

    const fn dynamic_node_has_following_static_at_parent(
        &self,
        raw: &[TemplateRawOp],
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

impl TemplateStorageStats {
    pub fn from_raw_ops(raw: &[TemplateRawOp]) -> Self {
        let mut stats = Self {
            raw_ops: raw.len(),
            ..Self::default()
        };
        let mut cursor = TemplateLoweringCursor::new();
        let mut anchors = Vec::new();
        let mut index = 0usize;

        while index < raw.len() {
            match raw[index] {
                TemplateRawOp::OpenElement { namespace, .. } => {
                    let has_namespace = namespace.is_some();
                    cursor.open_element(stats.ops, has_namespace);
                    stats.push_op();
                    stats.push_static();
                    if has_namespace {
                        stats.push_static();
                    }
                }
                TemplateRawOp::CloseElement => {
                    let _ = cursor.close_element();
                }
                TemplateRawOp::StaticAttr { namespace, .. } => {
                    stats.push_op();
                    stats.push_static();
                    stats.push_static();
                    if namespace.is_some() {
                        stats.push_static();
                    }
                }
                TemplateRawOp::DynamicAttr => {
                    let path = cursor.current_element_path();
                    if path.is_empty() {
                        stats.path_overflow = true;
                    }
                    stats.push_anchor(
                        &mut anchors,
                        cursor.current_element_op(),
                        TemplateSlotPath::append_children(path).bits(),
                    );
                }
                TemplateRawOp::StaticText { .. } => {
                    let _ = cursor.next_node_path();
                    stats.push_op();
                    stats.push_static();
                }
                TemplateRawOp::DynamicNode => match cursor.try_next_slot_path(raw, index) {
                    Ok(path) => {
                        stats.push_anchor(&mut anchors, cursor.node_anchor_op(), path.bits());
                    }
                    Err(()) => {
                        stats.path_overflow = true;
                        stats.push_anchor(&mut anchors, cursor.node_anchor_op(), 0);
                    }
                },
            }
            index += 1;
        }

        cursor.finish();
        stats.anchors = anchors.len();
        stats
    }

    fn push_op(&mut self) {
        self.ops += 1;
    }

    fn push_static(&mut self) {
        self.strings += 1;
        self.ops += 1;
    }

    fn push_anchor(&mut self, anchors: &mut Vec<AnchorStats>, op: u16, path: u128) {
        self.dynamic_values += 1;

        if let Some(last) = anchors.last_mut() {
            if last.same_anchor(op, path) {
                last.value_count += 1;
                return;
            }
        }

        if anchors.iter().any(|anchor| anchor.same_anchor(op, path)) {
            panic!("dynamic values for a template anchor must be contiguous");
        }

        anchors.push(AnchorStats {
            op,
            path,
            value_count: 1,
        });
    }

    pub const fn exceeds_storage_limits(self) -> bool {
        self.path_overflow
            || self.raw_ops > TEMPLATE_RAW_OPS_CAP
            || self.ops > TEMPLATE_STORAGE_OPS_CAP
            || self.strings > TEMPLATE_STORAGE_STRING_CAP
            || self.anchors > TEMPLATE_STORAGE_DYNAMIC_CAP
            || self.raw_ops > TEMPLATE_STORAGE_MAX_CAP
            || self.ops > TEMPLATE_STORAGE_MAX_CAP
            || self.strings > TEMPLATE_STORAGE_MAX_CAP
            || self.dynamic_values > u16::MAX as usize
    }

    pub fn max_required_chunks(self) -> usize {
        let chunks = required_chunks(self.raw_ops, TEMPLATE_RAW_OPS_CAP);
        let chunks = chunks.max(required_chunks(self.ops, TEMPLATE_STORAGE_OPS_CAP));
        let chunks = chunks.max(required_chunks(self.strings, TEMPLATE_STORAGE_STRING_CAP));
        let chunks = chunks.max(required_chunks(self.anchors, TEMPLATE_STORAGE_DYNAMIC_CAP));
        let chunks = chunks.max(required_chunks(
            self.raw_ops
                .max(self.ops)
                .max(self.strings)
                .max(self.dynamic_values),
            TEMPLATE_STORAGE_MAX_CAP,
        ));
        let chunks = chunks.max(required_chunks(self.dynamic_values, u16::MAX as usize));
        if self.path_overflow {
            chunks.max(2)
        } else {
            chunks
        }
    }
}

impl AnchorStats {
    fn same_anchor(self, op: u16, path: u128) -> bool {
        self.op == op && self.path == path
    }
}

const fn required_chunks(value: usize, limit: usize) -> usize {
    if value == 0 { 1 } else { value.div_ceil(limit) }
}

const fn tree_has_static_root_node(tree: &'static TemplateRawTree) -> bool {
    match tree {
        TemplateRawTree::Empty
        | TemplateRawTree::StaticAttr { .. }
        | TemplateRawTree::DynamicAttr
        | TemplateRawTree::DynamicNode => false,
        TemplateRawTree::Element { .. } | TemplateRawTree::StaticText(_) => true,
        TemplateRawTree::Sequence(children) => children_have_static_root_node(children, 0),
    }
}

const fn children_have_static_root_node(
    children: &'static [&'static TemplateRawTree],
    start: usize,
) -> bool {
    let mut index = start;
    while index < children.len() {
        if tree_has_static_root_node(children[index]) {
            return true;
        }
        index += 1;
    }

    false
}

const fn push_element_start<
    const OPS_CAP: usize,
    const STRING_CAP: usize,
    const DYNAMIC_CAP: usize,
>(
    storage: &mut TemplateStorage<OPS_CAP, STRING_CAP, DYNAMIC_CAP>,
    cursor: &mut TemplateLoweringCursor,
    tag: &'static str,
    namespace: Option<&'static str>,
) {
    let has_namespace = namespace.is_some();
    cursor.open_element(storage.ops_len(), has_namespace);
    storage.push_op(TemplateOp::enter(0, has_namespace));
    storage.push_static(tag);
    if let Some(namespace) = namespace {
        storage.push_static(namespace);
    }
}

const fn push_element_end<
    const OPS_CAP: usize,
    const STRING_CAP: usize,
    const DYNAMIC_CAP: usize,
>(
    storage: &mut TemplateStorage<OPS_CAP, STRING_CAP, DYNAMIC_CAP>,
    cursor: &mut TemplateLoweringCursor,
) {
    let (enter_index, namespace) = cursor.close_element();
    let skip = storage.ops_len() - enter_index;
    if skip > TemplateOp::MAX_CAP {
        panic!("template op skip exceeds packed op capacity");
    }
    storage.set_op(enter_index, TemplateOp::enter(skip as u16, namespace));
}

const fn push_static_attr<
    const OPS_CAP: usize,
    const STRING_CAP: usize,
    const DYNAMIC_CAP: usize,
>(
    storage: &mut TemplateStorage<OPS_CAP, STRING_CAP, DYNAMIC_CAP>,
    name: &'static str,
    value: &'static str,
    namespace: Option<&'static str>,
) {
    storage.push_op(TemplateOp::attr(namespace.is_some()));
    storage.push_static(name);
    storage.push_static(value);
    if let Some(namespace) = namespace {
        storage.push_static(namespace);
    }
}

const fn lower_raw_tree<const OPS_CAP: usize, const STRING_CAP: usize, const DYNAMIC_CAP: usize>(
    tree: &'static TemplateRawTree,
    storage: &mut TemplateStorage<OPS_CAP, STRING_CAP, DYNAMIC_CAP>,
    cursor: &mut TemplateLoweringCursor,
    following_static_at_parent: bool,
) {
    match tree {
        TemplateRawTree::Empty => {}
        TemplateRawTree::Sequence(children) => {
            let mut index = 0;
            while index < children.len() {
                lower_raw_tree(
                    children[index],
                    storage,
                    cursor,
                    following_static_at_parent
                        || children_have_static_root_node(children, index + 1),
                );
                index += 1;
            }
        }
        TemplateRawTree::Element {
            tag,
            namespace,
            attrs,
            children,
        } => {
            push_element_start(storage, cursor, tag, *namespace);
            lower_raw_tree(attrs, storage, cursor, false);
            lower_raw_tree(children, storage, cursor, false);
            push_element_end(storage, cursor);
        }
        TemplateRawTree::StaticAttr {
            name,
            value,
            namespace,
        } => {
            push_static_attr(storage, name, value, *namespace);
        }
        TemplateRawTree::DynamicAttr => {
            storage.push_anchor(
                cursor.current_element_op(),
                TemplateSlotPath::append_children(cursor.current_element_path()).bits(),
            );
        }
        TemplateRawTree::StaticText(value) => {
            let _ = cursor.next_node_path();
            storage.push_op(TemplateOp::text());
            storage.push_static(value);
        }
        TemplateRawTree::DynamicNode => {
            let path = cursor.next_slot_path_after_dynamic_node(following_static_at_parent);
            storage.push_anchor(cursor.node_anchor_op(), path.bits());
        }
    }
}

const fn lower_raw_template<
    const OPS_CAP: usize,
    const STRING_CAP: usize,
    const DYNAMIC_CAP: usize,
>(
    raw: &[TemplateRawOp],
    storage: &mut TemplateStorage<OPS_CAP, STRING_CAP, DYNAMIC_CAP>,
) {
    let mut cursor = TemplateLoweringCursor::new();
    let mut index = 0usize;
    while index < raw.len() {
        match raw[index] {
            TemplateRawOp::OpenElement { tag, namespace } => {
                push_element_start(storage, &mut cursor, tag, namespace);
            }
            TemplateRawOp::CloseElement => {
                push_element_end(storage, &mut cursor);
            }
            TemplateRawOp::StaticAttr {
                name,
                value,
                namespace,
            } => {
                push_static_attr(storage, name, value, namespace);
            }
            TemplateRawOp::DynamicAttr => {
                storage.push_anchor(
                    cursor.current_element_op(),
                    TemplateSlotPath::append_children(cursor.current_element_path()).bits(),
                );
            }
            TemplateRawOp::StaticText { value } => {
                let _ = cursor.next_node_path();
                storage.push_op(TemplateOp::text());
                storage.push_static(value);
            }
            TemplateRawOp::DynamicNode => {
                let path = cursor.next_slot_path(raw, index);
                storage.push_anchor(cursor.node_anchor_op(), path.bits());
            }
        }
        index += 1;
    }
    cursor.finish();
}

impl<const OPS_CAP: usize, const STRING_CAP: usize, const DYNAMIC_CAP: usize>
    TemplateStorage<OPS_CAP, STRING_CAP, DYNAMIC_CAP>
{
    /// Lower a raw template tape into packed storage in const context.
    pub const fn build(raw: &[TemplateRawOp]) -> Self {
        let mut storage = Self {
            ops: ConstVec::new_with_max_size(),
            strings: ConstVec::new_with_max_size(),
            anchors: ConstVec::new_with_max_size(),
        };

        lower_raw_template(raw, &mut storage);
        storage.sort_anchors_in_fill_order();
        storage
    }

    /// Lower a raw template tree into packed storage in const context.
    pub const fn build_from_tree(tree: &'static TemplateRawTree) -> Self {
        let mut storage = Self {
            ops: ConstVec::new_with_max_size(),
            strings: ConstVec::new_with_max_size(),
            anchors: ConstVec::new_with_max_size(),
        };
        let mut cursor = TemplateLoweringCursor::new();

        lower_raw_tree(tree, &mut storage, &mut cursor, false);
        cursor.finish();
        storage.sort_anchors_in_fill_order();
        storage
    }

    /// Return this storage as a compact template.
    pub const fn as_template(&'static self) -> Template {
        Template::new(
            self.ops.as_slice(),
            self.strings.as_slice(),
            self.anchors.as_slice(),
        )
    }

    /// Borrow the lowered packed ops.
    pub fn ops(&self) -> &[TemplateOp] {
        self.ops.as_slice()
    }

    /// Borrow the lowered static string pool.
    pub fn strings(&self) -> &[&'static str] {
        self.strings.as_slice()
    }

    /// Borrow the lowered dynamic anchors.
    pub fn anchors(&self) -> &[TemplateAnchor] {
        self.anchors.as_slice()
    }

    #[doc(hidden)]
    pub fn into_leaked_template(self) -> Template {
        Template::new(
            Box::leak(self.ops.as_slice().to_vec().into_boxed_slice()),
            Box::leak(self.strings.as_slice().to_vec().into_boxed_slice()),
            Box::leak(self.anchors.as_slice().to_vec().into_boxed_slice()),
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

    const fn push_op(&mut self, op: TemplateOp) {
        self.ops.push(op);
    }

    const fn set_op(&mut self, index: usize, op: TemplateOp) {
        self.ops.set(index, op);
    }

    const fn push_anchor(&mut self, op: u16, path: u128) {
        let len = self.anchors.len();
        if len > 0 {
            let last = self.anchors.at(len - 1);
            if last.same_anchor(op, path) {
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
            if self.anchors.at(i).same_anchor(op, path) {
                panic!("dynamic values for a template anchor must be contiguous");
            }
            i += 1;
        }
        let value_start = if len == 0 {
            0
        } else {
            let last = self.anchors.at(len - 1);
            last.value_start + last.value_count
        };
        let anchor = TemplateAnchor {
            op,
            path,
            value_start,
            value_count: 1,
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

impl Template {
    /// Lower a raw template tape into a leaked runtime template.
    pub fn from_raw_ops(raw: &'static [TemplateRawOp]) -> Self {
        TemplateStorage::<
            TEMPLATE_STORAGE_MAX_CAP,
            TEMPLATE_STORAGE_MAX_CAP,
            TEMPLATE_STORAGE_MAX_CAP,
        >::build(raw)
        .into_leaked_template()
    }
}
