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
/// the [`crate::TEMPLATE_SLOT_PATH_MAX_PATH_BITS`] slot-path payload limit, and
/// a path consumes at least one bit per nesting level (`TemplatePath::next_child`
/// shifts left by one). This cap matches the `u128` path width so the bit-width
/// splitter is the binding constraint and depths up to the slot-path payload
/// limit lower directly instead of hitting a "stack capacity exceeded" panic.
const TEMPLATE_PATH_STACK_CAP: usize = 128;

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
        };
        let mut cursor = TemplateLoweringCursor::new();

        lower_raw_tree(tree, &mut storage, &mut cursor, false);
        cursor.finish();
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

    /// Leak this storage into a compact runtime template.
    #[cfg(test)]
    pub(crate) fn into_leaked_template(self) -> Template {
        Template::new(
            Box::leak(self.ops.as_slice().to_vec().into_boxed_slice()),
            Box::leak(self.strings.as_slice().to_vec().into_boxed_slice()),
            Box::leak(self.anchors.as_slice().to_vec().into_boxed_slice()),
        )
    }
}

/// Lower a raw template tree into a leaked [`Template`] at runtime.
///
/// Runs the same lowering (`build_from_tree`/`lower_raw_tree`) the const path uses, but at runtime
/// instead of in const evaluation. The debug-only lazy template path uses this so dev builds skip
/// const-evaluating the optimized template for every `rsx!` site. The leak is bounded because
/// callers cache the result per template.
#[cfg(debug_assertions)]
pub fn build_runtime_template(tree: &'static TemplateRawTree) -> Template {
    let mut builder = RuntimeTemplateBuilder::new();
    lower_raw_tree_runtime(tree, &mut builder, false);
    builder.finish()
}

/// Runtime mirror of [`lower_raw_tree`] that drives the non-generic [`RuntimeTemplateBuilder`].
///
/// Mirrors the const lowering arm-for-arm so it produces the identical op tape, strings, anchors,
/// and value-kind hash, but without the capacity const generics - so the debug lazy path codegens
/// the lowering once instead of monomorphizing it per `(ops, strings, anchors)` capacity combo.
#[cfg(debug_assertions)]
fn lower_raw_tree_runtime(
    tree: &'static TemplateRawTree,
    builder: &mut RuntimeTemplateBuilder,
    following_static_at_parent: bool,
) {
    match tree {
        TemplateRawTree::Empty => {}
        TemplateRawTree::Sequence(children) => {
            let mut index = 0;
            while index < children.len() {
                lower_raw_tree_runtime(
                    children[index],
                    builder,
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
            builder.open_element(tag, *namespace);
            lower_raw_tree_runtime(attrs, builder, false);
            lower_raw_tree_runtime(children, builder, false);
            builder.close_element();
        }
        TemplateRawTree::StaticAttr {
            name,
            value,
            namespace,
        } => builder.static_attr(name, value, *namespace),
        TemplateRawTree::DynamicAttr => builder.dynamic_attr(),
        TemplateRawTree::StaticText(value) => builder.static_text(value),
        TemplateRawTree::DynamicNode => builder.dynamic_node(following_static_at_parent),
    }
}

#[derive(Clone, Copy)]
pub struct TemplateElementFrame {
    pub(crate) enter_index: usize,
    namespace: bool,
    pub(crate) path: TemplatePath,
}

pub struct TemplateLoweringCursor {
    enter_stack: [TemplateElementFrame; TEMPLATE_PATH_STACK_CAP],
    pub(crate) next_paths: [TemplatePath; TEMPLATE_PATH_STACK_CAP],
    last_static_paths: [TemplatePath; TEMPLATE_PATH_STACK_CAP],
    pub(crate) stack_pointer: usize,
}

impl TemplateLoweringCursor {
    pub const fn new() -> Self {
        let mut next_paths = [TemplatePath::empty(); TEMPLATE_PATH_STACK_CAP];
        next_paths[0] = TemplatePath::root(0);
        Self {
            enter_stack: [TemplateElementFrame {
                enter_index: 0,
                namespace: false,
                path: TemplatePath::empty(),
            }; TEMPLATE_PATH_STACK_CAP],
            next_paths,
            last_static_paths: [TemplatePath::empty(); TEMPLATE_PATH_STACK_CAP],
            stack_pointer: 0,
        }
    }

    pub const fn open_element(&mut self, enter_index: usize, namespace: bool) {
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
        self.last_static_paths[self.stack_pointer + 1] = TemplatePath::empty();
        self.last_static_paths[self.stack_pointer] = path;
        self.stack_pointer += 1;
    }

    pub const fn close_element(&mut self) -> TemplateElementFrame {
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
        self.current_element_frame().path
    }

    pub(crate) const fn node_anchor_parent_op_index(&self) -> u16 {
        if self.stack_pointer == 0 {
            ROOT_PARENT_OP_INDEX
        } else {
            self.current_element_frame().enter_index as u16
        }
    }

    pub(crate) const fn current_element_frame(&self) -> TemplateElementFrame {
        if self.stack_pointer == 0 {
            panic!("template cursor is not inside an element");
        }
        let frame = self.enter_stack[self.stack_pointer - 1];
        if frame.enter_index > TemplateOp::MAX_CAP {
            panic!("template enter op exceeds packed op capacity");
        }
        frame
    }

    pub const fn next_node_path(&mut self) -> TemplatePath {
        let path = self.next_paths[self.stack_pointer];
        self.next_paths[self.stack_pointer] = path.next_sibling();
        self.last_static_paths[self.stack_pointer] = path;
        path
    }

    const fn next_slot_path_after_dynamic_node(
        &self,
        has_following_static_at_parent: bool,
    ) -> TemplateSlotPath {
        if has_following_static_at_parent {
            return TemplateSlotPath::static_node(self.next_paths[self.stack_pointer]);
        }

        if self.stack_pointer > 0 {
            return TemplateSlotPath::last_static_node(self.current_element_path());
        }

        let last_static_path = self.last_static_paths[self.stack_pointer];
        TemplateSlotPath::last_static_node(last_static_path)
    }

    /// Return a structural root static anchor if the next root position needs one.
    pub fn static_root_anchor(&self) -> Option<(u16, TemplateSlotPath)> {
        let path = self.next_paths[self.stack_pointer];
        (self.stack_pointer == 0 && !path.is_empty())
            .then(|| (ROOT_PARENT_OP_INDEX, TemplateSlotPath::static_node(path)))
    }

    /// Return the current element's dynamic attribute anchor and whether its path overflowed.
    pub fn dynamic_attr_anchor(&self) -> (u16, TemplateSlotPath, bool) {
        let frame = self.current_element_frame();
        (
            frame.enter_index as u16,
            TemplateSlotPath::static_node(frame.path),
            frame.path.is_empty(),
        )
    }

    /// Return the next dynamic node anchor and whether its path overflowed.
    pub fn dynamic_node_anchor(
        &self,
        following_static_at_parent: bool,
    ) -> (u16, TemplateSlotPath, bool) {
        let parent_op_index = self.node_anchor_parent_op_index();
        if following_static_at_parent {
            let path = self.next_paths[self.stack_pointer];
            if path.is_empty() {
                return (
                    parent_op_index,
                    TemplateSlotPath::last_static_node(TemplatePath::empty()),
                    true,
                );
            }
            return (parent_op_index, TemplateSlotPath::static_node(path), false);
        }

        if self.stack_pointer > 0 {
            let path = self.current_element_path();
            if path.is_empty() {
                return (
                    parent_op_index,
                    TemplateSlotPath::last_static_node(TemplatePath::empty()),
                    true,
                );
            }
            return (
                parent_op_index,
                TemplateSlotPath::last_static_node(path),
                false,
            );
        }

        (
            parent_op_index,
            TemplateSlotPath::last_static_node(self.last_static_paths[self.stack_pointer]),
            false,
        )
    }

    pub const fn finish(&self) {
        if self.stack_pointer != 0 {
            panic!("template ended with unclosed elements");
        }
    }
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
        if ($cursor).stack_pointer == 0 {
            ($storage).push_static_anchor(
                ROOT_PARENT_OP_INDEX,
                TemplateSlotPath::static_node(($cursor).next_paths[0]),
            );
        }
        ($cursor).open_element(($storage).ops_len(), has_namespace);
        ($storage).push_op(TemplateOp::enter(0, has_namespace));
        ($storage).push_static($tag);
        if let Some(namespace) = namespace {
            ($storage).push_static(namespace);
        }
    }};
    (close_element($storage:expr, $cursor:expr)) => {{
        let frame = ($cursor).close_element();
        let enter_index = frame.enter_index;
        let namespace = frame.namespace;
        let skip = ($storage).ops_len() - enter_index;
        if skip > TemplateOp::MAX_CAP {
            panic!("template op skip exceeds packed op capacity");
        }
        ($storage).set_op(enter_index, TemplateOp::enter(skip as u16, namespace));
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
        if ($cursor).stack_pointer == 0 {
            ($storage).push_static_anchor(
                ROOT_PARENT_OP_INDEX,
                TemplateSlotPath::static_node(($cursor).next_paths[0]),
            );
        }
        let _ = ($cursor).next_node_path();
        ($storage).push_op(TemplateOp::text());
        ($storage).push_static($value);
    }};
    (dynamic_node($storage:expr, $cursor:expr, $following_static_at_parent:expr)) => {{
        let path = ($cursor).next_slot_path_after_dynamic_node($following_static_at_parent);
        ($storage).push_anchor(($cursor).node_anchor_parent_op_index(), path, false);
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

        $($constness)? fn push_anchor(
            &mut self,
            parent_op_index: u16,
            path: TemplateSlotPath,
            is_attr: bool,
        ) {
            let len = self.anchors.len();
            if len > 0 {
                let last = self.anchors.at(len - 1);
                if last.same_anchor(parent_op_index, path) {
                    if is_attr {
                        if last.attr_end == u16::MAX {
                            panic!("anchor overflow");
                        }
                        self.anchors.set(
                            len - 1,
                            TemplateAnchor {
                                attr_end: last.attr_end + 1,
                                ..last
                            },
                        );
                    } else {
                        if last.node_end == u16::MAX {
                            panic!("anchor overflow");
                        }
                        self.anchors.set(
                            len - 1,
                            TemplateAnchor {
                                node_end: last.node_end + 1,
                                ..last
                            },
                        );
                    }
                    return;
                }
            }

            let (node_start, attr_start) = if len == 0 {
                (0, 0)
            } else {
                let last = self.anchors.at(len - 1);
                (last.node_end, last.attr_end)
            };
            if (is_attr && attr_start == u16::MAX) || (!is_attr && node_start == u16::MAX) {
                panic!("anchor overflow");
            }
            self.anchors.push(TemplateAnchor {
                parent_op_index,
                path,
                node_start,
                node_end: node_start + (!is_attr as u16),
                attr_start,
                attr_end: attr_start + (is_attr as u16),
            });
        }

        $($constness)? fn push_static_anchor(
            &mut self,
            parent_op_index: u16,
            path: TemplateSlotPath,
        ) {
            let len = self.anchors.len();
            if len > 0 {
                let last = self.anchors.at(len - 1);
                if last.same_anchor(parent_op_index, path) {
                    return;
                }
            }

            let (node_start, attr_start) = if len == 0 {
                (0, 0)
            } else {
                let last = self.anchors.at(len - 1);
                (last.node_end, last.attr_end)
            };
            self.anchors.push(TemplateAnchor {
                parent_op_index,
                path,
                node_start,
                node_end: node_start,
                attr_start,
                attr_end: attr_start,
            });
        }

    };
}

impl<const OPS_CAP: usize, const STRING_CAP: usize, const DYNAMIC_CAP: usize>
    TemplateStorage<OPS_CAP, STRING_CAP, DYNAMIC_CAP>
{
    template_storage_methods!(const);
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
            let frame = cursor.current_element_frame();
            let path = TemplateSlotPath::static_node(frame.path);
            storage.push_anchor(frame.enter_index as u16, path, true);
        }
        TemplateRawTree::StaticText(value) => {
            template_lowering!(static_text(storage, cursor, value));
        }
        TemplateRawTree::DynamicNode => {
            template_lowering!(dynamic_node(storage, cursor, following_static_at_parent));
        }
    }
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
    fn new() -> Self {
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
        let frame = self.cursor.current_element_frame();
        let path = TemplateSlotPath::static_node(frame.path);
        self.storage
            .push_anchor(frame.enter_index as u16, path, true);
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
    pub fn finish(self) -> Template {
        self.cursor.finish();
        self.storage.into_leaked_template()
    }
}

impl RuntimeTemplateStorage {
    fn into_leaked_template(self) -> Template {
        Template::new(
            Box::leak(self.ops.into_boxed_slice()),
            Box::leak(self.strings.into_boxed_slice()),
            Box::leak(self.anchors.into_boxed_slice()),
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

    fn anchor_parts(template: Template) -> Vec<(u16, u128, u16, u16, u16, u16)> {
        template
            .anchors()
            .iter()
            .map(|anchor| {
                (
                    anchor.parent_op_index,
                    anchor.path.bits(),
                    anchor.node_start,
                    anchor.node_end,
                    anchor.attr_start,
                    anchor.attr_end,
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
        assert_eq!(template.anchors()[0].nodes(), 0..2);
    }
}
