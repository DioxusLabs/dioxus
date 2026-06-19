use dioxus_const_vec::ConstVec;

use super::anchor::ROOT_PARENT_OP_INDEX;
use super::{Template, TemplateAnchor, TemplatePath, TemplateRawTree, TemplateSlotPath};
use crate::op::TemplateOp;

/// Maximum packed template storage capacity.
pub const TEMPLATE_STORAGE_MAX_CAP: usize = TemplateOp::MAX_CAP;

/// Default packed template operation storage capacity.
pub const TEMPLATE_STORAGE_OPS_CAP: usize = 128;

/// Default static string storage capacity.
pub const TEMPLATE_STORAGE_STRING_CAP: usize = 128;

/// Default dynamic anchor storage capacity.
pub const TEMPLATE_STORAGE_DYNAMIC_CAP: usize = 16;

/// Maximum element nesting depth handled by a single template chunk.
///
/// The rsx splitter wraps subtrees in synthetic boundaries once a path exceeds
/// its bit-width limit (`TEMPLATE_PATH_BITS_SPLIT_LIMIT`, currently 96), and a
/// path consumes at least one bit per nesting level (`TemplatePath::next_child`
/// shifts left by one). So a chunk that reaches the splitter's lowering can nest
/// no deeper than that bit limit. This cap is kept comfortably above it (and at
/// the `u128` path width) so the bit-width splitter is always the binding
/// constraint — depths between the old cap and the bit limit lower directly
/// instead of hitting an opaque "stack capacity exceeded" panic.
const TEMPLATE_PATH_STACK_CAP: usize = 128;

/// Storage requirements for lowering a template.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct TemplateStorageStats {
    /// Number of packed template operations.
    pub ops: usize,
    /// Number of static strings.
    pub strings: usize,
    /// Number of dynamic anchors.
    pub anchors: usize,
    /// Number of runtime dynamic values.
    pub dynamic_values: usize,
    /// Whether lowering overflowed the path stack.
    pub path_overflow: bool,
}

#[derive(Clone, Copy)]
struct AnchorStats {
    parent_op_index: u16,
    path: u128,
    value_count: usize,
}

/// Const storage for a template.
#[derive(Clone, Copy)]
pub struct TemplateStorage<
    const OPS_CAP: usize = TEMPLATE_STORAGE_OPS_CAP,
    const STRING_CAP: usize = TEMPLATE_STORAGE_STRING_CAP,
    const DYNAMIC_CAP: usize = TEMPLATE_STORAGE_DYNAMIC_CAP,
> {
    ops: ConstVec<TemplateOp, OPS_CAP>,
    strings: ConstVec<&'static str, STRING_CAP>,
    anchors: ConstVec<TemplateAnchor, DYNAMIC_CAP>,
    /// Running hash of each dynamic value's kind (attribute vs node) in fill
    /// order. Folded into the template hash so kind-incompatible templates that
    /// share an op tape compare unequal; never stored on the template itself.
    value_kind_hash: u64,
}

#[derive(Clone, Copy)]
struct TemplateElementFrame {
    enter_index: usize,
    namespace: bool,
    path: TemplatePath,
    dynamic_attrs: usize,
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
                dynamic_attrs: 0,
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
            dynamic_attrs: 0,
        };
        self.next_paths[self.stack_pointer + 1] = path.next_child();
        self.stack_pointer += 1;
    }

    const fn close_element(&mut self) -> TemplateElementFrame {
        if self.stack_pointer == 0 {
            panic!("template close op without matching open op");
        }
        self.stack_pointer -= 1;
        self.enter_stack[self.stack_pointer]
    }

    const fn defer_dynamic_attr(&mut self) {
        if self.stack_pointer == 0 {
            panic!("dynamic attr raw op without an open element");
        }
        let index = self.stack_pointer - 1;
        let frame = self.enter_stack[index];
        self.enter_stack[index] = TemplateElementFrame {
            dynamic_attrs: frame.dynamic_attrs + 1,
            ..frame
        };
    }

    const fn current_element_path(&self) -> TemplatePath {
        if self.stack_pointer == 0 {
            panic!("dynamic attr raw op without an open element");
        }
        self.current_element_frame().path
    }

    const fn node_anchor_parent_op_index(&self) -> u16 {
        if self.stack_pointer == 0 {
            ROOT_PARENT_OP_INDEX
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
            panic!("template ended with unclosed elements");
        }
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
    pub fn new() -> Self {
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
        self.stats
            .flush_dynamic_attrs(&mut self.anchors, &self.cursor);
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
        self.cursor.defer_dynamic_attr();
    }

    /// Count a static text node.
    pub fn static_text(&mut self) {
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
                    path.bits(),
                );
            }
            Err(()) => {
                self.stats.path_overflow = true;
                self.stats.push_anchor(
                    &mut self.anchors,
                    self.cursor.node_anchor_parent_op_index(),
                    0,
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

impl TemplateStorageStats {
    fn push_op(&mut self) {
        self.ops += 1;
    }

    fn push_static(&mut self) {
        self.strings += 1;
        self.ops += 1;
    }

    fn push_anchor(&mut self, anchors: &mut Vec<AnchorStats>, parent_op_index: u16, path: u128) {
        self.dynamic_values += 1;

        if let Some(last) = anchors.last_mut() {
            if last.same_anchor(parent_op_index, path) {
                last.value_count += 1;
                return;
            }
        }

        if anchors
            .iter()
            .any(|anchor| anchor.same_anchor(parent_op_index, path))
        {
            panic!("anchor gap");
        }

        anchors.push(AnchorStats {
            parent_op_index,
            path,
            value_count: 1,
        });
    }

    fn flush_dynamic_attrs(
        &mut self,
        anchors: &mut Vec<AnchorStats>,
        cursor: &TemplateLoweringCursor,
    ) {
        let frame = cursor.current_element_frame();
        let path = frame.path;
        if path.is_empty() {
            self.path_overflow = true;
        }
        let path = TemplateSlotPath::append_children(path).bits();
        for _ in 0..frame.dynamic_attrs {
            self.push_anchor(anchors, frame.enter_index as u16, path);
        }
    }

    pub const fn exceeds_storage_limits(self) -> bool {
        self.path_overflow
            || self.ops > TEMPLATE_STORAGE_OPS_CAP
            || self.strings > TEMPLATE_STORAGE_STRING_CAP
            || self.anchors > TEMPLATE_STORAGE_DYNAMIC_CAP
            || self.ops > TEMPLATE_STORAGE_MAX_CAP
            || self.strings > TEMPLATE_STORAGE_MAX_CAP
            || self.dynamic_values > u16::MAX as usize
    }

    pub fn max_required_chunks(self) -> usize {
        let chunks = required_chunks(self.ops, TEMPLATE_STORAGE_OPS_CAP);
        let chunks = chunks.max(required_chunks(self.strings, TEMPLATE_STORAGE_STRING_CAP));
        let chunks = chunks.max(required_chunks(self.anchors, TEMPLATE_STORAGE_DYNAMIC_CAP));
        let chunks = chunks.max(required_chunks(
            self.ops.max(self.strings).max(self.dynamic_values),
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
    fn same_anchor(self, parent_op_index: u16, path: u128) -> bool {
        self.parent_op_index == parent_op_index && self.path == path
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

// Replace this macro with a const trait once const trait methods are stable enough for this shared
// lowering path.
macro_rules! template_lowering {
    (open_element($storage:expr, $cursor:expr, $tag:expr, $namespace:expr)) => {{
        let namespace = $namespace;
        let has_namespace = namespace.is_some();
        ($cursor).open_element(($storage).ops_len(), has_namespace);
        ($storage).push_op(TemplateOp::enter(0, has_namespace));
        ($storage).push_static($tag);
        if let Some(namespace) = namespace {
            ($storage).push_static(namespace);
        }
    }};
    (close_element($storage:expr, $cursor:expr)) => {{
        template_lowering!(dynamic_attrs($storage, $cursor));
        let frame = ($cursor).close_element();
        let enter_index = frame.enter_index;
        let namespace = frame.namespace;
        let skip = ($storage).ops_len() - enter_index;
        if skip > TemplateOp::MAX_CAP {
            panic!("template op skip exceeds packed op capacity");
        }
        ($storage).set_op(enter_index, TemplateOp::enter(skip as u16, namespace));
    }};
    (dynamic_attrs($storage:expr, $cursor:expr)) => {{
        let frame = ($cursor).current_element_frame();
        let path = TemplateSlotPath::append_children(frame.path).bits();
        let mut index = 0;
        while index < frame.dynamic_attrs {
            ($storage).push_anchor(frame.enter_index as u16, path, true);
            index += 1;
        }
    }};
    (static_attr($storage:expr, $name:expr, $value:expr, $namespace:expr)) => {{
        let namespace = $namespace;
        ($storage).push_op(TemplateOp::attr(namespace.is_some()));
        ($storage).push_static($name);
        ($storage).push_static($value);
        if let Some(namespace) = namespace {
            ($storage).push_static(namespace);
        }
    }};
    (static_text($storage:expr, $cursor:expr, $value:expr)) => {{
        let _ = ($cursor).next_node_path();
        ($storage).push_op(TemplateOp::text());
        ($storage).push_static($value);
    }};
    (dynamic_node($storage:expr, $cursor:expr, $following_static_at_parent:expr)) => {{
        let path = ($cursor).next_slot_path_after_dynamic_node($following_static_at_parent);
        ($storage).push_anchor(($cursor).node_anchor_parent_op_index(), path.bits(), false);
    }};
}

macro_rules! template_storage_methods {
    ($($constness:tt)?) => {
        $($constness)? fn push_static(&mut self, value: &'static str) {
            let id = self.strings.len();
            if id >= TemplateOp::MAX_CAP {
                panic!("static op id exceeds packed op capacity");
            }
            self.strings.push(value);
            self.push_op(TemplateOp::static_text(id as u16));
        }

        $($constness)? fn ops_len(&self) -> usize {
            self.ops.len()
        }

        $($constness)? fn push_op(&mut self, op: TemplateOp) {
            if self.ops.len() >= TemplateOp::MAX_CAP {
                panic!("template ops exceed packed op capacity");
            }
            self.ops.push(op);
        }

        $($constness)? fn set_op(&mut self, index: usize, op: TemplateOp) {
            self.ops.set(index, op);
        }

        $($constness)? fn push_anchor(&mut self, parent_op_index: u16, path: u128, is_attr: bool) {
            // Fold this value's kind into the running signature in fill order
            // (one value per call, including merges into the previous anchor).
            // The template hash mixes this in so an attribute slot and a node
            // slot at the same anchor never produce equal templates.
            self.value_kind_hash =
                xxhash_rust::const_xxh64::xxh64(&[is_attr as u8], self.value_kind_hash);

            let len = self.anchors.len();
            if len > 0 {
                let last = self.anchors.at(len - 1);
                if last.same_anchor(parent_op_index, path) {
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
                if self.anchors.at(i).same_anchor(parent_op_index, path) {
                    panic!("anchor gap");
                }
                i += 1;
            }

            let value_start = if len == 0 {
                0
            } else {
                let last = self.anchors.at(len - 1);
                last.value_start + last.value_count
            };
            self.anchors.push(TemplateAnchor {
                parent_op_index,
                path,
                value_start,
                value_count: 1,
            });
        }

        $($constness)? fn sort_anchors_in_fill_order(&mut self) {
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
    };
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
    template_lowering!(open_element(storage, cursor, tag, namespace));
}

const fn push_element_end<
    const OPS_CAP: usize,
    const STRING_CAP: usize,
    const DYNAMIC_CAP: usize,
>(
    storage: &mut TemplateStorage<OPS_CAP, STRING_CAP, DYNAMIC_CAP>,
    cursor: &mut TemplateLoweringCursor,
) {
    template_lowering!(close_element(storage, cursor));
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
    template_lowering!(static_attr(storage, name, value, namespace));
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
            cursor.defer_dynamic_attr();
        }
        TemplateRawTree::StaticText(value) => {
            template_lowering!(static_text(storage, cursor, value));
        }
        TemplateRawTree::DynamicNode => {
            template_lowering!(dynamic_node(storage, cursor, following_static_at_parent));
        }
    }
}

impl<const OPS_CAP: usize, const STRING_CAP: usize, const DYNAMIC_CAP: usize>
    TemplateStorage<OPS_CAP, STRING_CAP, DYNAMIC_CAP>
{
    /// Build storage from a template tree.
    pub const fn build_from_tree(tree: &'static TemplateRawTree) -> Self {
        let mut storage = Self {
            ops: ConstVec::new_with_max_size(),
            strings: ConstVec::new_with_max_size(),
            anchors: ConstVec::new_with_max_size(),
            value_kind_hash: 0,
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
            self.value_kind_hash,
        )
    }

    /// Leak this storage into a compact runtime template.
    pub fn into_leaked_template(self) -> Template {
        Template::new(
            Box::leak(self.ops.as_slice().to_vec().into_boxed_slice()),
            Box::leak(self.strings.as_slice().to_vec().into_boxed_slice()),
            Box::leak(self.anchors.as_slice().to_vec().into_boxed_slice()),
            self.value_kind_hash,
        )
    }

    template_storage_methods!(const);
}

/// Builds a leaked runtime template directly from semantic template events.
pub struct RuntimeTemplateBuilder {
    storage: RuntimeTemplateStorage,
    cursor: TemplateLoweringCursor,
}

#[derive(Default)]
struct RuntimeTemplateStorage {
    ops: RuntimeTemplateVec<TemplateOp>,
    strings: RuntimeTemplateVec<&'static str>,
    anchors: RuntimeTemplateVec<TemplateAnchor>,
    /// Running hash of each dynamic value's kind (attribute vs node) in fill
    /// order. See [`TemplateStorage::value_kind_hash`].
    value_kind_hash: u64,
}

struct RuntimeTemplateVec<T>(Vec<T>);

impl<T> Default for RuntimeTemplateVec<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T: Copy> RuntimeTemplateVec<T> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn push(&mut self, value: T) {
        self.0.push(value);
    }

    fn set(&mut self, index: usize, value: T) {
        self.0[index] = value;
    }

    fn at(&self, index: usize) -> T {
        self.0[index]
    }

    fn swap(&mut self, a: usize, b: usize) {
        self.0.swap(a, b);
    }

    fn into_boxed_slice(self) -> Box<[T]> {
        self.0.into_boxed_slice()
    }
}

impl Default for RuntimeTemplateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeTemplateBuilder {
    /// Create a new runtime template builder.
    pub fn new() -> Self {
        Self {
            storage: RuntimeTemplateStorage::default(),
            cursor: TemplateLoweringCursor::new(),
        }
    }

    /// Emit an element start.
    pub fn open_element(&mut self, tag: &'static str, namespace: Option<&'static str>) {
        template_lowering!(open_element(
            &mut self.storage,
            &mut self.cursor,
            tag,
            namespace
        ));
    }

    /// Emit the end of the current element.
    pub fn close_element(&mut self) {
        template_lowering!(close_element(&mut self.storage, &mut self.cursor));
    }

    /// Emit a static attribute.
    pub fn static_attr(
        &mut self,
        name: &'static str,
        value: &'static str,
        namespace: Option<&'static str>,
    ) {
        template_lowering!(static_attr(&mut self.storage, name, value, namespace));
    }

    /// Emit a dynamic attribute slot on the current element.
    pub fn dynamic_attr(&mut self) {
        self.cursor.defer_dynamic_attr();
    }

    /// Emit a static text node.
    pub fn static_text(&mut self, value: &'static str) {
        template_lowering!(static_text(&mut self.storage, &mut self.cursor, value));
    }

    /// Emit a dynamic node slot.
    pub fn dynamic_node(&mut self, following_static_at_parent: bool) {
        template_lowering!(dynamic_node(
            &mut self.storage,
            &mut self.cursor,
            following_static_at_parent
        ));
    }

    /// Finish this builder and return a leaked template.
    pub fn finish(mut self) -> Template {
        self.cursor.finish();
        self.storage.sort_anchors_in_fill_order();
        self.storage.into_leaked_template()
    }
}

impl RuntimeTemplateStorage {
    fn into_leaked_template(self) -> Template {
        Template::new(
            Box::leak(self.ops.into_boxed_slice()),
            Box::leak(self.strings.into_boxed_slice()),
            Box::leak(self.anchors.into_boxed_slice()),
            self.value_kind_hash,
        )
    }

    template_storage_methods!();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template_from_tree(tree: &'static TemplateRawTree) -> Template {
        TemplateStorage::<64, 64, 16>::build_from_tree(tree).into_leaked_template()
    }

    fn anchor_parts(template: Template) -> Vec<(u16, u128, u16, u16)> {
        template
            .anchors()
            .iter()
            .map(|anchor| {
                (
                    anchor.parent_op_index,
                    anchor.path,
                    anchor.value_start,
                    anchor.value_count,
                )
            })
            .collect()
    }

    fn assert_same_template(actual: Template, expected: Template) {
        assert_eq!(
            actual.decoded_ops().collect::<Vec<_>>(),
            expected.decoded_ops().collect::<Vec<_>>()
        );
        assert_eq!(actual.strings(), expected.strings());
        assert_eq!(anchor_parts(actual), anchor_parts(expected));
    }

    #[test]
    fn runtime_builder_matches_tree_for_nested_namespaces_and_dynamic_attrs() {
        static ATTR: TemplateRawTree = TemplateRawTree::StaticAttr {
            name: "fill",
            value: "red",
            namespace: Some("style"),
        };
        static ATTRS: [&TemplateRawTree; 2] = [&ATTR, &TemplateRawTree::DynamicAttr];
        static ATTRS_TREE: TemplateRawTree = TemplateRawTree::Sequence(&ATTRS);
        static TEXT: TemplateRawTree = TemplateRawTree::StaticText("hello");
        static INNER_CHILDREN: [&TemplateRawTree; 1] = [&TemplateRawTree::DynamicNode];
        static INNER_CHILDREN_TREE: TemplateRawTree = TemplateRawTree::Sequence(&INNER_CHILDREN);
        static INNER: TemplateRawTree = TemplateRawTree::Element {
            tag: "span",
            namespace: None,
            attrs: &TemplateRawTree::Empty,
            children: &INNER_CHILDREN_TREE,
        };
        static CHILDREN: [&TemplateRawTree; 2] = [&TEXT, &INNER];
        static CHILDREN_TREE: TemplateRawTree = TemplateRawTree::Sequence(&CHILDREN);
        static TREE: TemplateRawTree = TemplateRawTree::Element {
            tag: "svg",
            namespace: Some("svg"),
            attrs: &ATTRS_TREE,
            children: &CHILDREN_TREE,
        };

        let mut builder = RuntimeTemplateBuilder::new();
        builder.open_element("svg", Some("svg"));
        builder.static_attr("fill", "red", Some("style"));
        builder.dynamic_attr();
        builder.static_text("hello");
        builder.open_element("span", None);
        builder.dynamic_node(false);
        builder.close_element();
        builder.close_element();

        assert_same_template(builder.finish(), template_from_tree(&TREE));
    }

    #[test]
    fn runtime_builder_places_dynamic_nodes_before_static_siblings() {
        static TEXT: TemplateRawTree = TemplateRawTree::StaticText("after");
        static CHILDREN: [&TemplateRawTree; 2] = [&TemplateRawTree::DynamicNode, &TEXT];
        static TREE: TemplateRawTree = TemplateRawTree::Sequence(&CHILDREN);

        let mut builder = RuntimeTemplateBuilder::new();
        builder.dynamic_node(true);
        builder.static_text("after");

        assert_same_template(builder.finish(), template_from_tree(&TREE));
    }

    #[test]
    fn runtime_builder_groups_adjacent_trailing_dynamic_nodes() {
        static CHILDREN: [&TemplateRawTree; 2] =
            [&TemplateRawTree::DynamicNode, &TemplateRawTree::DynamicNode];
        static TREE: TemplateRawTree = TemplateRawTree::Sequence(&CHILDREN);

        let mut builder = RuntimeTemplateBuilder::new();
        builder.dynamic_node(false);
        builder.dynamic_node(false);

        let template = builder.finish();
        assert_same_template(template, template_from_tree(&TREE));
        assert_eq!(template.anchors()[0].values(), 0..2);
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
        assert_eq!(stats.dynamic_values, template.dynamic_value_count());
    }
}
