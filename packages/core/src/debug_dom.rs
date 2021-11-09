use crate::{innerlude::ScopeInner, virtual_dom::VirtualDom, VNode};

impl std::fmt::Display for VirtualDom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let base = self.base_scope();
        let root = base.root_node();

        let renderer = ScopeRenderer {
            show_fragments: false,
            skip_components: false,

            _scope: base,
            _pre_render: false,
            _newline: true,
            _indent: true,
            _max_depth: usize::MAX,
        };

        renderer.render(self, root, f, 0)
    }
}

/// render the scope to a string using the rsx! syntax
pub(crate) struct ScopeRenderer<'a> {
    pub skip_components: bool,
    pub show_fragments: bool,
    pub _scope: &'a ScopeInner,
    pub _pre_render: bool,
    pub _newline: bool,
    pub _indent: bool,
    pub _max_depth: usize,
}

// this is more or less a debug tool, but it'll render the entire tree to the terminal
impl<'a> ScopeRenderer<'a> {
    pub fn render(
        &self,
        vdom: &VirtualDom,
        node: &VNode,
        f: &mut std::fmt::Formatter,
        il: u16,
    ) -> std::fmt::Result {
        const INDENT: &str = "    ";
        let write_indent = |_f: &mut std::fmt::Formatter, le| {
            for _ in 0..le {
                write!(_f, "{}", INDENT).unwrap();
            }
        };

        match &node {
            VNode::Text(text) => {
                write_indent(f, il);
                writeln!(f, "\"{}\"", text.text)?
            }
            VNode::Anchor(_anchor) => {
                write_indent(f, il);
                writeln!(f, "Anchor {{}}")?;
            }
            VNode::Element(el) => {
                write_indent(f, il);
                writeln!(f, "{} {{", el.tag_name)?;
                // write!(f, "element: {}", el.tag_name)?;
                let mut attr_iter = el.attributes.iter().peekable();

                while let Some(attr) = attr_iter.next() {
                    match attr.namespace {
                        None => {
                            //
                            write_indent(f, il + 1);
                            writeln!(f, "{}: \"{}\"", attr.name, attr.value)?
                        }

                        Some(ns) => {
                            // write the opening tag
                            write_indent(f, il + 1);
                            write!(f, " {}:\"", ns)?;
                            let mut cur_ns_el = attr;
                            'ns_parse: loop {
                                write!(f, "{}:{};", cur_ns_el.name, cur_ns_el.value)?;
                                match attr_iter.peek() {
                                    Some(next_attr) if next_attr.namespace == Some(ns) => {
                                        cur_ns_el = attr_iter.next().unwrap();
                                    }
                                    _ => break 'ns_parse,
                                }
                            }
                            // write the closing tag
                            write!(f, "\"")?;
                        }
                    }
                }

                for child in el.children {
                    self.render(vdom, child, f, il + 1)?;
                }
                write_indent(f, il);

                writeln!(f, "}}")?;
            }
            VNode::Fragment(frag) => {
                if self.show_fragments {
                    write_indent(f, il);
                    writeln!(f, "Fragment {{")?;
                    for child in frag.children {
                        self.render(vdom, child, f, il + 1)?;
                    }
                    write_indent(f, il);
                    writeln!(f, "}}")?;
                } else {
                    for child in frag.children {
                        self.render(vdom, child, f, il)?;
                    }
                }
            }
            VNode::Component(vcomp) => {
                let idx = vcomp.associated_scope.get().unwrap();
                if !self.skip_components {
                    let new_node = vdom.get_scope(idx).unwrap().root_node();
                    self.render(vdom, new_node, f, il)?;
                }
            }
            VNode::Suspended { .. } => {
                // we can't do anything with suspended nodes
            }
        }
        Ok(())
    }
}
