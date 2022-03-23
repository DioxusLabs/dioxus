use std::collections::{HashMap, HashSet, VecDeque};

use dioxus_core::{ElementId, Mutations, VNode, VirtualDom};
pub mod layout;
pub mod layout_attributes;

/// A tree that can sync with dioxus mutations backed by a hashmap.
/// Intended for use in lazy native renderers with a state that passes from parrent to children and or accumulates state from children to parrents.
/// To get started implement [PushedDownState] and or [BubbledUpState] and call [Tree::apply_mutations] and [Tree::update_state].
#[derive(Debug)]

pub struct Tree<US: BubbledUpState = (), DS: PushedDownState = ()> {
    pub root: usize,
    pub nodes: Vec<Option<TreeNode<US, DS>>>,
    pub nodes_listening: HashMap<&'static str, HashSet<usize>>,
}

impl<US: BubbledUpState, DS: PushedDownState> Tree<US, DS> {
    pub fn new() -> Tree<US, DS> {
        Tree {
            root: 0,
            nodes: {
                let mut v = Vec::new();
                v.push(Some(TreeNode::new(
                    0,
                    TreeNodeType::Element {
                        tag: "Root".to_string(),
                        namespace: Some("Root"),
                        children: Vec::new(),
                    },
                )));
                v
            },
            nodes_listening: HashMap::new(),
        }
    }

    /// Updates the tree, up and down state and return a set of nodes that were updated
    pub fn apply_mutations(&mut self, mutations_vec: Vec<Mutations>) -> Vec<usize> {
        let mut nodes_updated = Vec::new();
        for mutations in mutations_vec {
            let mut node_stack: smallvec::SmallVec<[usize; 5]> = smallvec::SmallVec::new();
            for e in mutations.edits {
                use dioxus_core::DomEdit::*;
                match e {
                    PushRoot { root } => node_stack.push(root as usize),
                    AppendChildren { many } => {
                        let target = if node_stack.len() >= many as usize + 1 {
                            *node_stack
                                .get(node_stack.len() - (many as usize + 1))
                                .unwrap()
                        } else {
                            0
                        };
                        for ns in node_stack.drain(node_stack.len() - many as usize..).rev() {
                            self.link_child(ns, target).unwrap();
                            nodes_updated.push(ns);
                        }
                    }
                    ReplaceWith { root, m } => {
                        let root = self.remove(root as usize).unwrap();
                        let target = root.parent.unwrap().0;
                        for ns in node_stack.drain(0..m as usize) {
                            nodes_updated.push(ns);
                            self.link_child(ns, target).unwrap();
                        }
                    }
                    InsertAfter { root, n } => {
                        let target = self.get(root as usize).parent.unwrap().0;
                        for ns in node_stack.drain(0..n as usize) {
                            nodes_updated.push(ns);
                            self.link_child(ns, target).unwrap();
                        }
                    }
                    InsertBefore { root, n } => {
                        let target = self.get(root as usize).parent.unwrap().0;
                        for ns in node_stack.drain(0..n as usize) {
                            nodes_updated.push(ns);
                            self.link_child(ns, target).unwrap();
                        }
                    }
                    Remove { root } => {
                        if let Some(parent) = self.get(root as usize).parent {
                            nodes_updated.push(parent.0);
                        }
                        self.remove(root as usize).unwrap();
                    }
                    CreateTextNode { root, text } => {
                        let n = TreeNode::new(
                            root,
                            TreeNodeType::Text {
                                text: text.to_string(),
                            },
                        );
                        self.insert(n);
                        node_stack.push(root as usize)
                    }
                    CreateElement { root, tag } => {
                        let n = TreeNode::new(
                            root,
                            TreeNodeType::Element {
                                tag: tag.to_string(),
                                namespace: None,
                                children: Vec::new(),
                            },
                        );
                        self.insert(n);
                        node_stack.push(root as usize)
                    }
                    CreateElementNs { root, tag, ns } => {
                        let n = TreeNode::new(
                            root,
                            TreeNodeType::Element {
                                tag: tag.to_string(),
                                namespace: Some(ns),
                                children: Vec::new(),
                            },
                        );
                        self.insert(n);
                        node_stack.push(root as usize)
                    }
                    CreatePlaceholder { root } => {
                        let n = TreeNode::new(root, TreeNodeType::Placeholder);
                        self.insert(n);
                        node_stack.push(root as usize)
                    }

                    NewEventListener {
                        event_name,
                        scope: _,
                        root,
                    } => {
                        if let Some(v) = self.nodes_listening.get_mut(event_name) {
                            v.insert(root as usize);
                        } else {
                            let mut hs = HashSet::new();
                            hs.insert(root as usize);
                            self.nodes_listening.insert(event_name, hs);
                        }
                    }
                    RemoveEventListener { root, event } => {
                        let v = self.nodes_listening.get_mut(event).unwrap();
                        v.remove(&(root as usize));
                    }
                    SetText {
                        root,
                        text: new_text,
                    } => {
                        let target = self.get_mut(root as usize);
                        nodes_updated.push(root as usize);
                        match &mut target.node_type {
                            TreeNodeType::Text { text } => {
                                *text = new_text.to_string();
                            }
                            _ => unreachable!(),
                        }
                    }
                    SetAttribute { root, .. } => {
                        nodes_updated.push(root as usize);
                    }
                    RemoveAttribute { root, .. } => {
                        nodes_updated.push(root as usize);
                    }
                }
            }
        }

        nodes_updated
    }

    pub fn update_state(
        &mut self,
        vdom: &VirtualDom,
        nodes_updated: Vec<usize>,
        us_ctx: &mut US::Ctx,
        ds_ctx: &mut DS::Ctx,
    ) -> Option<HashSet<usize>> {
        let mut to_rerender = HashSet::new();
        let mut nodes_updated: Vec<_> = nodes_updated
            .into_iter()
            .map(|id| (id, self.get(id).height))
            .collect();
        nodes_updated.dedup();
        nodes_updated.sort_by_key(|(_, h)| *h);

        // bubble up state. To avoid calling reduce more times than nessisary start from the bottom and go up.
        // todo: this is called multable times per element?
        let mut to_bubble: VecDeque<_> = nodes_updated.clone().into();
        while let Some((id, height)) = to_bubble.pop_back() {
            let node = self.get_mut(id as usize);
            let vnode = node.element(vdom);
            let node_type = &node.node_type;
            let up_state = &mut node.up_state;
            let children = match node_type {
                TreeNodeType::Element { children, .. } => Some(children),
                _ => None,
            };
            // todo: reduce cloning state
            let old = up_state.clone();
            let mut new = up_state.clone();
            let parent = node.parent.clone();
            new.reduce(
                children
                    .unwrap_or(&Vec::new())
                    .clone()
                    .iter()
                    .map(|c| &self.get(c.0).up_state),
                vnode,
                us_ctx,
            );
            if new != old {
                to_rerender.insert(id);
                if let Some(p) = parent {
                    let i = to_bubble.partition_point(|(_, h)| *h < height - 1);
                    // make sure the parent is not already queued
                    if i >= to_bubble.len() || to_bubble.get(i).unwrap().0 != p.0 {
                        to_bubble.insert(i, (p.0, height - 1));
                    }
                }
                let node = self.get_mut(id as usize);
                node.up_state = new;
            }
        }

        // push down state. To avoid calling reduce more times than nessisary start from the top and go down.
        let mut to_push: VecDeque<_> = nodes_updated.clone().into();
        while let Some((id, height)) = to_push.pop_front() {
            let node = self.get_mut(id as usize);
            // todo: reduce cloning state
            let old = node.down_state.clone();
            let mut new = node.down_state.clone();
            let vnode = node.element(vdom);
            new.reduce(
                node.parent.map(|e| &self.get(e.0).down_state),
                vnode,
                ds_ctx,
            );
            if new != old {
                to_rerender.insert(id);
                let node = self.get_mut(id as usize);
                match &node.node_type {
                    TreeNodeType::Element { children, .. } => {
                        for c in children {
                            let i = to_bubble.partition_point(|(_, h)| *h < height + 1);
                            to_bubble.insert(i, (c.0, height + 1));
                        }
                    }
                    _ => (),
                };
                node.down_state = new;
            }
        }

        Some(to_rerender)
    }

    fn link_child(&mut self, child_id: usize, parent_id: usize) -> Option<()> {
        debug_assert_ne!(child_id, parent_id);
        let parent = self.get_mut(parent_id);
        parent.add_child(ElementId(child_id));
        let parent_height = parent.height + 1;
        self.get_mut(child_id).set_parent(ElementId(parent_id));
        self.increase_height(child_id, parent_height);
        Some(())
    }

    fn increase_height(&mut self, id: usize, amount: u16) {
        let n = self.get_mut(id);
        n.height += amount;
        match &n.node_type {
            TreeNodeType::Element { children, .. } => {
                for c in children.clone() {
                    self.increase_height(c.0, amount);
                }
            }
            _ => (),
        }
    }

    fn remove(&mut self, id: usize) -> Option<TreeNode<US, DS>> {
        let mut node = self.nodes.get_mut(id as usize).unwrap().take().unwrap();
        match &mut node.node_type {
            TreeNodeType::Element { children, .. } => {
                for c in children {
                    self.remove(c.0).unwrap();
                }
            }
            _ => (),
        }
        Some(node)
    }

    fn insert(&mut self, node: TreeNode<US, DS>) {
        let current_len = self.nodes.len();
        let id = node.id.0;
        if current_len - 1 < node.id.0 {
            // self.nodes.reserve(1 + id - current_len);
            self.nodes.extend((0..1 + id - current_len).map(|_| None));
        }
        self.nodes[id] = Some(node);
    }

    pub fn get(&self, id: usize) -> &TreeNode<US, DS> {
        self.nodes.get(id).unwrap().as_ref().unwrap()
    }

    fn get_mut(&mut self, id: usize) -> &mut TreeNode<US, DS> {
        self.nodes.get_mut(id).unwrap().as_mut().unwrap()
    }

    pub fn get_listening_sorted(&self, event: &'static str) -> Vec<&TreeNode<US, DS>> {
        if let Some(nodes) = self.nodes_listening.get(event) {
            let mut listening: Vec<_> = nodes.iter().map(|id| self.get(*id)).collect();
            listening.sort_by(|n1, n2| (n1.height).cmp(&n2.height).reverse());
            listening
        } else {
            Vec::new()
        }
    }
}

/// The node is stored client side and stores render data
#[derive(Debug, Clone)]
pub struct TreeNode<US: BubbledUpState, DS: PushedDownState> {
    pub id: ElementId,
    pub parent: Option<ElementId>,
    pub up_state: US,
    pub down_state: DS,
    pub node_type: TreeNodeType,
    pub height: u16,
}

#[derive(Debug, Clone)]
pub enum TreeNodeType {
    Text {
        text: String,
    },
    Element {
        tag: String,
        namespace: Option<&'static str>,
        children: Vec<ElementId>,
    },
    Placeholder,
}

impl<US: BubbledUpState, DS: PushedDownState> TreeNode<US, DS> {
    fn new(id: u64, node_type: TreeNodeType) -> Self {
        TreeNode {
            id: ElementId(id as usize),
            parent: None,
            node_type,
            down_state: DS::default(),
            up_state: US::default(),
            height: 0,
        }
    }

    fn element<'b>(&self, vdom: &'b VirtualDom) -> &'b VNode<'b> {
        vdom.get_element(self.id).unwrap()
    }

    fn add_child(&mut self, child: ElementId) {
        match &mut self.node_type {
            TreeNodeType::Element { children, .. } => {
                children.push(child);
            }
            _ => (),
        }
    }

    fn set_parent(&mut self, parent: ElementId) {
        self.parent = Some(parent);
    }
}

/// This state that is passed down to children. For example text properties (`<b>` `<i>` `<u>`) would be passed to children.
/// Called when the current node's node properties are modified or a parrent's [PushedDownState] is modified.
/// Called at most once per update.
pub trait PushedDownState: Default + PartialEq + Clone {
    type Ctx;
    fn reduce(&mut self, parent: Option<&Self>, vnode: &VNode, ctx: &mut Self::Ctx);
}
impl PushedDownState for () {
    type Ctx = ();
    fn reduce(&mut self, _parent: Option<&Self>, _vnode: &VNode, _ctx: &mut Self::Ctx) {}
}

/// This state is derived from children. For example a non-flexbox div's size could be derived from the size of children.
/// Called when the current node's node properties are modified, a child's [BubbledUpState] is modified or a child is removed.
/// Called at most once per update.
pub trait BubbledUpState: Default + PartialEq + Clone {
    type Ctx;
    fn reduce<'a, I>(&mut self, children: I, vnode: &VNode, ctx: &mut Self::Ctx)
    where
        I: Iterator<Item = &'a Self>,
        Self: 'a;
}
impl BubbledUpState for () {
    type Ctx = ();
    fn reduce<'a, I>(&mut self, _children: I, _vnode: &VNode, _ctx: &mut Self::Ctx)
    where
        I: Iterator<Item = &'a Self>,
        Self: 'a,
    {
    }
}

// /// The nodes that need to be updated after updating a state.
// pub struct Update {
//     children: bool,
//     parent: bool,
// }

// /// This state is derived from children and parents.
// /// Called when the current node's node properties are modified or a parent or child's [State] is modified.
// /// Unlike [BubbledUpState] and [PushedDownState] this may be called mulable times per update. Prefer those over this.
// pub trait State: Default + PartialEq + Clone {
//     fn reduce<'a, I>(&mut self, parent: Option<&Self>, children: I, vnode: &VNode) -> Update
//     where
//         I: Iterator<Item = &'a Self>,
//         Self: 'a;
// }
// impl State for () {
//     fn reduce<'a, I>(&mut self, _parent: Option<&Self>, _children: I, _vnode: &VNode) -> Update
//     where
//         I: Iterator<Item = &'a Self>,
//         Self: 'a,
//     {
//         Update {
//             children: false,
//             parent: false,
//         }
//     }
// }

#[test]
fn test_insert() {
    use dioxus_core::*;
    use dioxus_core_macro::*;
    use dioxus_html as dioxus_elements;

    #[derive(Debug, Default, PartialEq, Clone)]
    struct Rect {
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    }

    impl BubbledUpState for Rect {
        type Ctx = ();

        fn reduce<'a, I>(&mut self, children: I, vnode: &VNode, _ctx: &mut Self::Ctx)
        where
            I: Iterator<Item = &'a Self>,
            Self: 'a,
        {
            match vnode {
                VNode::Text(t) => {
                    *self = Rect {
                        x: 0,
                        y: 0,
                        width: t.text.len().try_into().unwrap(),
                        height: 1,
                    };
                    return;
                }
                _ => (),
            }
            self.width = 2;
            self.height = 2;
            for c in children {
                println!("\t{c:?}");
                self.width = self.width.max(c.width);
                self.height += c.height;
            }
        }
    }

    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        rsx!(cx, div {})
    }

    let vdom = VirtualDom::new(Base);
    let node_1 = rsx! {
        div{
            div{
                "hello"
                "hello world"
            }
        }
    };
    let node_2 = rsx! {
        div{
            div{
                "hello"
                "hello world"
            }
        }
    };
    let mutations = vdom.diff_lazynodes(node_1, node_2);

    let mut tree: Tree<Rect, ()> = Tree {
        root: 0,
        nodes: {
            let mut v = Vec::new();
            v.push(Some(TreeNode::new(
                0,
                TreeNodeType::Element {
                    tag: "Root".to_string(),
                    namespace: Some("Root"),
                    children: Vec::new(),
                },
            )));
            v
        },
        nodes_listening: HashMap::new(),
    };
    println!("{:?}", mutations);
    let to_update = tree.apply_mutations(vec![mutations.0]);
    let to_rerender = tree
        .update_state(&vdom, to_update, &mut (), &mut ())
        .unwrap();
    println!("{to_rerender:?}");
    panic!("{}", format!("{:?}", &tree.nodes[1..]).replace("\\", ""));
}
