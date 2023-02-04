use dioxus::prelude::*;
use dioxus_native_core::node::NodeType;
use dioxus_native_core::prelude::*;
use std::any::TypeId;
use std::sync::{Arc, Mutex};

fn create_blank_element() -> NodeType {
    NodeType::Element(ElementNode {
        tag: "div".to_owned(),
        namespace: None,
        attributes: HashMap::new(),
        listeners: HashMap::new(),
    })
}

#[test]
fn node_pass() {
    #[derive(Debug, Default, Clone, PartialEq)]
    struct Number(i32);

    impl Pass for Number {
        type ChildDependencies = ();
        type NodeDependencies = ();
        type ParentDependencies = ();
        const NODE_MASK: NodeMaskBuilder = NodeMaskBuilder::new();

        fn pass<'a>(
            &mut self,
            node_view: NodeView,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> bool {
            self.0 += 1;
            true
        }

        fn create<'a>(
            node_view: NodeView<()>,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> Self {
            let mut myself = Self::default();
            myself.pass(node_view, node, parent, children, context);
            myself
        }
    }

    let mut tree: RealDom = RealDom::new(Box::new([Number::to_type_erased()]));
    tree.update_state(SendAnyMap::new(), false);

    assert_eq!(tree.get(tree.root_id()).unwrap().get(), Some(&Number(1)));

    // mark the node as dirty
    tree.get_mut(tree.root_id()).unwrap().get_mut::<Number>();

    tree.update_state(SendAnyMap::new(), false);
    assert_eq!(tree.get(tree.root_id()).unwrap().get(), Some(&Number(2)));
}

#[test]
fn dependant_node_pass() {
    #[derive(Debug, Default, Clone, PartialEq)]
    struct AddNumber(i32);

    impl Pass for AddNumber {
        type ChildDependencies = ();
        type NodeDependencies = (SubtractNumber,);
        type ParentDependencies = ();
        const NODE_MASK: NodeMaskBuilder = NodeMaskBuilder::new();

        fn pass<'a>(
            &mut self,
            node_view: NodeView,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> bool {
            self.0 += 1;
            true
        }

        fn create<'a>(
            node_view: NodeView<()>,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> Self {
            let mut myself = Self::default();
            myself.pass(node_view, node, parent, children, context);
            myself
        }
    }

    #[derive(Debug, Default, Clone, PartialEq)]
    struct SubtractNumber(i32);

    impl Pass for SubtractNumber {
        type ChildDependencies = ();
        type NodeDependencies = ();
        type ParentDependencies = ();
        const NODE_MASK: NodeMaskBuilder = NodeMaskBuilder::new();

        fn pass<'a>(
            &mut self,
            node_view: NodeView,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> bool {
            self.0 -= 1;
            true
        }

        fn create<'a>(
            node_view: NodeView<()>,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> Self {
            let mut myself = Self::default();
            myself.pass(node_view, node, parent, children, context);
            myself
        }
    }

    let mut tree: RealDom = RealDom::new(Box::new([
        AddNumber::to_type_erased(),
        SubtractNumber::to_type_erased(),
    ]));
    tree.update_state(SendAnyMap::new(), false);

    let root = tree.get(tree.root_id()).unwrap();
    assert_eq!(root.get(), Some(&AddNumber(1)));
    assert_eq!(root.get(), Some(&SubtractNumber(-1)));

    // mark the subtract state as dirty, it should update the add state
    tree.get_mut(tree.root_id())
        .unwrap()
        .get_mut::<SubtractNumber>();

    tree.update_state(SendAnyMap::new(), false);
    let root = tree.get(tree.root_id()).unwrap();
    assert_eq!(root.get(), Some(&AddNumber(2)));
    assert_eq!(root.get(), Some(&SubtractNumber(-2)));

    // mark the add state as dirty, it should ~not~ update the subtract state
    tree.get_mut(tree.root_id()).unwrap().get_mut::<AddNumber>();

    tree.update_state(SendAnyMap::new(), false);
    let root = tree.get(tree.root_id()).unwrap();
    assert_eq!(root.get(), Some(&AddNumber(3)));
    assert_eq!(root.get(), Some(&SubtractNumber(-2)));
}

#[test]
fn independant_node_pass() {
    #[derive(Debug, Default, Clone, PartialEq)]
    struct AddNumber(i32);

    impl Pass for AddNumber {
        type ChildDependencies = ();
        type NodeDependencies = ();
        type ParentDependencies = ();

        const NODE_MASK: NodeMaskBuilder = NodeMaskBuilder::new();

        fn pass<'a>(
            &mut self,
            node_view: NodeView,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> bool {
            self.0 += 1;
            true
        }

        fn create<'a>(
            node_view: NodeView<()>,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> Self {
            let mut myself = Self::default();
            myself.pass(node_view, node, parent, children, context);
            myself
        }
    }

    #[derive(Debug, Default, Clone, PartialEq)]
    struct SubtractNumber(i32);

    impl Pass for SubtractNumber {
        type ChildDependencies = ();
        type NodeDependencies = ();
        type ParentDependencies = ();

        const NODE_MASK: NodeMaskBuilder = NodeMaskBuilder::new();

        fn pass<'a>(
            &mut self,
            node_view: NodeView,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> bool {
            self.0 -= 1;
            true
        }

        fn create<'a>(
            node_view: NodeView<()>,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> Self {
            let mut myself = Self::default();
            myself.pass(node_view, node, parent, children, context);
            myself
        }
    }

    let mut tree: RealDom = RealDom::new(Box::new([
        AddNumber::to_type_erased(),
        SubtractNumber::to_type_erased(),
    ]));
    tree.update_state(SendAnyMap::new(), false);

    let root = tree.get(tree.root_id()).unwrap();
    assert_eq!(root.get(), Some(&AddNumber(1)));
    assert_eq!(root.get(), Some(&SubtractNumber(-1)));

    // mark the subtract state as dirty, it should ~not~ update the add state
    tree.get_mut(tree.root_id())
        .unwrap()
        .get_mut::<SubtractNumber>();

    tree.update_state(SendAnyMap::new(), false);

    let root = tree.get(tree.root_id()).unwrap();
    assert_eq!(root.get(), Some(&AddNumber(1)));
    assert_eq!(root.get(), Some(&SubtractNumber(-2)));

    // mark the add state as dirty, it should ~not~ update the subtract state
    tree.get_mut(tree.root_id()).unwrap().get_mut::<AddNumber>();

    tree.update_state(SendAnyMap::new(), false);

    let root = tree.get(tree.root_id()).unwrap();
    assert_eq!(root.get(), Some(&AddNumber(2)));
    assert_eq!(root.get(), Some(&SubtractNumber(-2)));
}

#[test]
fn down_pass() {
    #[derive(Debug, Clone, PartialEq)]
    struct AddNumber(i32);

    impl Default for AddNumber {
        fn default() -> Self {
            Self(1)
        }
    }

    impl Pass for AddNumber {
        type ChildDependencies = ();
        type NodeDependencies = ();
        type ParentDependencies = (AddNumber,);

        const NODE_MASK: NodeMaskBuilder = NodeMaskBuilder::new();

        fn pass<'a>(
            &mut self,
            node_view: NodeView,
            node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
            parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
            children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
            context: &SendAnyMap,
        ) -> bool {
            if let Some((parent,)) = parent {
                *self.0 += *parent.0;
            }
            true
        }
    }

    let mut tree: RealDom = RealDom::new(Box::new([AddNumber::to_type_erased()]));
    let parent = tree.root_id();
    let child1 = tree.create_node(create_blank_element(), true);
    tree.add_child(parent, child1);
    let grandchild1 = tree.create_node(create_blank_element(), true);
    tree.add_child(child1, grandchild1);
    let child2 = tree.create_node(create_blank_element(), true);
    tree.add_child(parent, child2);
    let grandchild2 = tree.create_node(create_blank_element(), true);
    tree.add_child(child2, grandchild2);

    tree.dirty_nodes
        .insert(TypeId::of::<AddNumber>(), NodeId(0));
    tree.update_state(SendAnyMap::new(), false);

    assert_eq!(tree.get(tree.root_id()).unwrap().state.add_number.0, 1);
    assert_eq!(tree.get(child1).unwrap().state.add_number.0, 2);
    assert_eq!(tree.get(grandchild1).unwrap().state.add_number.0, 3);
    assert_eq!(tree.get(child2).unwrap().state.add_number.0, 2);
    assert_eq!(tree.get(grandchild2).unwrap().state.add_number.0, 3);
}

// #[test]
// fn dependant_down_pass() {
//     // 0
//     let mut tree = Tree::new(1);
//     let parent = tree.root_id();
//     // 1
//     let child1 = tree.create_node(1);
//     tree.add_child(parent, child1);
//     // 2
//     let grandchild1 = tree.create_node(1);
//     tree.add_child(child1, grandchild1);
//     // 3
//     let child2 = tree.create_node(1);
//     tree.add_child(parent, child2);
//     // 4
//     let grandchild2 = tree.create_node(1);
//     tree.add_child(child2, grandchild2);

//     struct AddPass;
//     impl Pass for AddPass {
//         fn pass_id(&self) -> PassId {
//             PassId(0)
//         }

//         fn dependancies(&self) -> &'static [PassId] {
//             &[PassId(1)]
//         }

//         fn dependants(&self) -> &'static [PassId] {
//             &[]
//         }

//         fn mask(&self) -> MemberMask {
//             MemberMask(0)
//         }
//     }
//     impl DownwardPass<i32> for AddPass {
//         fn pass(&self, node: &mut i32, parent: Option<&mut i32>, _: &SendAnyMap) -> PassReturn {
//             if let Some(parent) = parent {
//                 *node += *parent;
//             } else {
//             }
//             PassReturn {
//                 progress: true,
//                 mark_dirty: true,
//             }
//         }
//     }

//     struct SubtractPass;
//     impl Pass for SubtractPass {
//         fn pass_id(&self) -> PassId {
//             PassId(1)
//         }

//         fn dependancies(&self) -> &'static [PassId] {
//             &[]
//         }

//         fn dependants(&self) -> &'static [PassId] {
//             &[PassId(0)]
//         }

//         fn mask(&self) -> MemberMask {
//             MemberMask(0)
//         }
//     }
//     impl DownwardPass<i32> for SubtractPass {
//         fn pass(&self, node: &mut i32, parent: Option<&mut i32>, _: &SendAnyMap) -> PassReturn {
//             if let Some(parent) = parent {
//                 *node -= *parent;
//             } else {
//             }
//             PassReturn {
//                 progress: true,
//                 mark_dirty: true,
//             }
//         }
//     }

//     let add_pass = AnyPass::Downward(&AddPass);
//     let subtract_pass = AnyPass::Downward(&SubtractPass);
//     let passes = vec![&add_pass, &subtract_pass];
//     let dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
//     dirty_nodes.insert(PassId(1), tree.root_id());
//     resolve_passes(&mut tree, dirty_nodes, passes, SendAnyMap::new());

//     // Tree before:
//     // 1=\
//     //   1=\
//     //     1
//     //   1=\
//     //     1
//     // Tree after subtract:
//     // 1=\
//     //   0=\
//     //     1
//     //   0=\
//     //     1
//     // Tree after add:
//     // 1=\
//     //   1=\
//     //     2
//     //   1=\
//     //     2
//     assert_eq!(tree.get(tree.root_id()).unwrap(), &1);
//     assert_eq!(tree.get(child1).unwrap(), &1);
//     assert_eq!(tree.get(grandchild1).unwrap(), &2);
//     assert_eq!(tree.get(child2).unwrap(), &1);
//     assert_eq!(tree.get(grandchild2).unwrap(), &2);
// }

// #[test]
// fn up_pass() {
//     // Tree before:
//     // 0=\
//     //   0=\
//     //     1
//     //   0=\
//     //     1
//     // Tree after:
//     // 2=\
//     //   1=\
//     //     1
//     //   1=\
//     //     1
//     let mut tree = Tree::new(0);
//     let parent = tree.root_id();
//     let child1 = tree.create_node(0);
//     tree.add_child(parent, child1);
//     let grandchild1 = tree.create_node(1);
//     tree.add_child(child1, grandchild1);
//     let child2 = tree.create_node(0);
//     tree.add_child(parent, child2);
//     let grandchild2 = tree.create_node(1);
//     tree.add_child(child2, grandchild2);

//     struct AddPass;
//     impl Pass for AddPass {
//         fn pass_id(&self) -> PassId {
//             PassId(0)
//         }

//         fn dependancies(&self) -> &'static [PassId] {
//             &[]
//         }

//         fn dependants(&self) -> &'static [PassId] {
//             &[]
//         }

//         fn mask(&self) -> MemberMask {
//             MemberMask(0)
//         }
//     }
//     impl UpwardPass<i32> for AddPass {
//         fn pass<'a>(
//             &self,
//             node: &mut i32,
//             children: &mut dyn Iterator<Item = &'a mut i32>,
//             _: &SendAnyMap,
//         ) -> PassReturn {
//             *node += children.map(|i| *i).sum::<i32>();
//             PassReturn {
//                 progress: true,
//                 mark_dirty: true,
//             }
//         }
//     }

//     let add_pass = AnyPass::Upward(&AddPass);
//     let passes = vec![&add_pass];
//     let dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
//     dirty_nodes.insert(PassId(0), grandchild1);
//     dirty_nodes.insert(PassId(0), grandchild2);
//     resolve_passes(&mut tree, dirty_nodes, passes, SendAnyMap::new());

//     assert_eq!(tree.get(tree.root_id()).unwrap(), &2);
//     assert_eq!(tree.get(child1).unwrap(), &1);
//     assert_eq!(tree.get(grandchild1).unwrap(), &1);
//     assert_eq!(tree.get(child2).unwrap(), &1);
//     assert_eq!(tree.get(grandchild2).unwrap(), &1);
// }

// #[test]
// fn dependant_up_pass() {
//     // 0
//     let mut tree = Tree::new(0);
//     let parent = tree.root_id();
//     // 1
//     let child1 = tree.create_node(0);
//     tree.add_child(parent, child1);
//     // 2
//     let grandchild1 = tree.create_node(1);
//     tree.add_child(child1, grandchild1);
//     // 3
//     let child2 = tree.create_node(0);
//     tree.add_child(parent, child2);
//     // 4
//     let grandchild2 = tree.create_node(1);
//     tree.add_child(child2, grandchild2);

//     struct AddPass;
//     impl Pass for AddPass {
//         fn pass_id(&self) -> PassId {
//             PassId(0)
//         }

//         fn dependancies(&self) -> &'static [PassId] {
//             &[PassId(1)]
//         }

//         fn dependants(&self) -> &'static [PassId] {
//             &[]
//         }

//         fn mask(&self) -> MemberMask {
//             MemberMask(0)
//         }
//     }
//     impl UpwardPass<i32> for AddPass {
//         fn pass<'a>(
//             &self,
//             node: &mut i32,
//             children: &mut dyn Iterator<Item = &'a mut i32>,
//             _: &SendAnyMap,
//         ) -> PassReturn {
//             *node += children.map(|i| *i).sum::<i32>();
//             PassReturn {
//                 progress: true,
//                 mark_dirty: true,
//             }
//         }
//     }

//     struct SubtractPass;
//     impl Pass for SubtractPass {
//         fn pass_id(&self) -> PassId {
//             PassId(1)
//         }

//         fn dependancies(&self) -> &'static [PassId] {
//             &[]
//         }

//         fn dependants(&self) -> &'static [PassId] {
//             &[PassId(0)]
//         }

//         fn mask(&self) -> MemberMask {
//             MemberMask(0)
//         }
//     }
//     impl UpwardPass<i32> for SubtractPass {
//         fn pass<'a>(
//             &self,
//             node: &mut i32,
//             children: &mut dyn Iterator<Item = &'a mut i32>,
//             _: &SendAnyMap,
//         ) -> PassReturn {
//             *node -= children.map(|i| *i).sum::<i32>();
//             PassReturn {
//                 progress: true,
//                 mark_dirty: true,
//             }
//         }
//     }

//     let add_pass = AnyPass::Upward(&AddPass);
//     let subtract_pass = AnyPass::Upward(&SubtractPass);
//     let passes = vec![&add_pass, &subtract_pass];
//     let dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
//     dirty_nodes.insert(PassId(1), grandchild1);
//     dirty_nodes.insert(PassId(1), grandchild2);
//     resolve_passes(&mut tree, dirty_nodes, passes, SendAnyMap::new());

//     // Tree before:
//     // 0=\
//     //   0=\
//     //     1
//     //   0=\
//     //     1
//     // Tree after subtract:
//     // 2=\
//     //   -1=\
//     //      1
//     //   -1=\
//     //      1
//     // Tree after add:
//     // 2=\
//     //   0=\
//     //     1
//     //   0=\
//     //     1
//     assert_eq!(tree.get(tree.root_id()).unwrap(), &2);
//     assert_eq!(tree.get(child1).unwrap(), &0);
//     assert_eq!(tree.get(grandchild1).unwrap(), &1);
//     assert_eq!(tree.get(child2).unwrap(), &0);
//     assert_eq!(tree.get(grandchild2).unwrap(), &1);
// }
