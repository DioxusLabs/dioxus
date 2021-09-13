use crate::innerlude::*;

pub(crate) struct ScopeRenderer<'a> {
    pub skip_components: bool,
    pub show_fragments: bool,
    pub _scope: &'a Scope,
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
                write!(f, "\"{}\"\n", text.text)?
            }
            VNode::Anchor(anchor) => {
                write_indent(f, il);
                write!(f, "Anchor {{}}\n")?;
            }
            VNode::Element(el) => {
                write_indent(f, il);
                write!(f, "{} {{\n", el.tag_name)?;
                // write!(f, "element: {}", el.tag_name)?;
                let mut attr_iter = el.attributes.iter().peekable();

                while let Some(attr) = attr_iter.next() {
                    match attr.namespace {
                        None => {
                            //
                            write_indent(f, il + 1);
                            write!(f, "{}: \"{}\"\n", attr.name, attr.value)?
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

                write!(f, "}}\n")?;
            }
            VNode::Fragment(frag) => {
                if self.show_fragments {
                    write_indent(f, il);
                    write!(f, "Fragment {{\n")?;
                    for child in frag.children {
                        self.render(vdom, child, f, il + 1)?;
                    }
                    write_indent(f, il);
                    write!(f, "}}\n")?;
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
