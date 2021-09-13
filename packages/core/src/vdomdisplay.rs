use crate::innerlude::*;

// this is more or less a debug tool, but it'll render the entire tree to the terminal
impl std::fmt::Display for VirtualDom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct ScopeRenderer<'a> {
            scope: &'a Scope,
            cfg: Cfg,
        }

        struct Cfg {
            pre_render: bool,
            newline: bool,
            indent: bool,
            max_depth: usize,
            skip_components: bool,
            show_fragments: bool,
        }

        impl<'a> ScopeRenderer<'a> {
            fn html_render(
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
                            self.html_render(vdom, child, f, il + 1)?;
                        }
                        write_indent(f, il);

                        write!(f, "}}\n")?;
                    }
                    VNode::Fragment(frag) => {
                        if self.cfg.show_fragments {
                            write_indent(f, il);
                            write!(f, "Fragment {{\n")?;
                            for child in frag.children {
                                self.html_render(vdom, child, f, il + 1)?;
                            }
                            write_indent(f, il);
                            write!(f, "}}\n")?;
                        } else {
                            for child in frag.children {
                                self.html_render(vdom, child, f, il)?;
                            }
                        }
                    }
                    VNode::Component(vcomp) => {
                        let idx = vcomp.associated_scope.get().unwrap();
                        if !self.cfg.skip_components {
                            let new_node = vdom.get_scope(idx).unwrap().root_node();
                            self.html_render(vdom, new_node, f, il)?;
                        }
                    }
                    VNode::Suspended { .. } => {
                        // we can't do anything with suspended nodes
                    }
                }
                Ok(())
            }
        }

        let base = self.base_scope();
        let root = base.root_node();
        let renderer = ScopeRenderer {
            scope: base,
            cfg: Cfg {
                show_fragments: false,
                pre_render: false,
                newline: true,
                indent: true,
                max_depth: usize::MAX,
                skip_components: false,
            },
        };

        renderer.html_render(self, root, f, 0)
    }
}
