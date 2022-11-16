// use dioxus_core::VirtualDom;

// use crate::config::SsrConfig;

// pub struct SsrRenderer {
//     vdom: VirtualDom,
//     cfg: SsrConfig,
// }

// impl Default for SsrRenderer {
//     fn default() -> Self {
//         Self::new(SsrConfig::default())
//     }
// }

// impl SsrRenderer {
//     pub fn new(cfg: SsrConfig) -> Self {
//         Self {
//             vdom: VirtualDom::new(app),
//             cfg,
//         }
//     }

//     pub fn render_lazy<'a>(&'a mut self, f: LazyNodes<'a, '_>) -> String {
//         let scope = self.vdom.base_scope();
//         let root = f.call(scope);
//         format!(
//             "{:}",
//             TextRenderer {
//                 cfg: self.cfg.clone(),
//                 root: &root,
//                 vdom: Some(&self.vdom),
//             }
//         )
//     }

//     fn html_render(
//         &self,
//         node: &VNode,
//         f: &mut impl Write,
//         il: u16,
//         last_node_was_text: &mut bool,
//     ) -> std::fmt::Result {
//         // match &node {
//         //     VNode::Text(text) => {
//         //         if *last_node_was_text {
//         //             write!(f, "<!--spacer-->")?;
//         //         }

//         //         if self.cfg.indent {
//         //             for _ in 0..il {
//         //                 write!(f, "    ")?;
//         //             }
//         //         }

//         //         *last_node_was_text = true;

//         //         write!(f, "{}", text.text)?
//         //     }
//         //     VNode::Element(el) => {
//         //         *last_node_was_text = false;

//         //         if self.cfg.indent {
//         //             for _ in 0..il {
//         //                 write!(f, "    ")?;
//         //             }
//         //         }

//         //         write!(f, "<{}", el.tag)?;

//         //         let inner_html = render_attributes(el.attributes.iter(), f)?;

//         //         match self.cfg.newline {
//         //             true => writeln!(f, ">")?,
//         //             false => write!(f, ">")?,
//         //         }

//         //         if let Some(inner_html) = inner_html {
//         //             write!(f, "{}", inner_html)?;
//         //         } else {
//         //             let mut last_node_was_text = false;
//         //             for child in el.children {
//         //                 self.html_render(child, f, il + 1, &mut last_node_was_text)?;
//         //             }
//         //         }

//         //         if self.cfg.newline {
//         //             writeln!(f)?;
//         //         }
//         //         if self.cfg.indent {
//         //             for _ in 0..il {
//         //                 write!(f, "    ")?;
//         //             }
//         //         }

//         //         write!(f, "</{}>", el.tag)?;
//         //         if self.cfg.newline {
//         //             writeln!(f)?;
//         //         }
//         //     }
//         //     VNode::Fragment(frag) => match frag.children.len() {
//         //         0 => {
//         //             *last_node_was_text = false;
//         //             if self.cfg.indent {
//         //                 for _ in 0..il {
//         //                     write!(f, "    ")?;
//         //                 }
//         //             }
//         //             write!(f, "<!--placeholder-->")?;
//         //         }
//         //         _ => {
//         //             for child in frag.children {
//         //                 self.html_render(child, f, il + 1, last_node_was_text)?;
//         //             }
//         //         }
//         //     },
//         //     VNode::Component(vcomp) => {
//         //         let idx = vcomp.scope.get().unwrap();

//         //         if let (Some(vdom), false) = (self.vdom, self.cfg.skip_components) {
//         //             let new_node = vdom.get_scope(idx).unwrap().root_node();
//         //             self.html_render(new_node, f, il + 1, last_node_was_text)?;
//         //         } else {
//         //         }
//         //     }
//         //     VNode::Template(t) => {
//         //         if let Some(vdom) = self.vdom {
//         //             todo!()
//         //         } else {
//         //             panic!("Cannot render template without vdom");
//         //         }
//         //     }
//         //     VNode::Placeholder(_) => {
//         //         todo!()
//         //     }
//         // }
//         Ok(())
//     }
// }

// impl<'a: 'c, 'c> Display for SsrRenderer<'a, '_, 'c> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         let mut last_node_was_text = false;
//         self.html_render(self.root, f, 0, &mut last_node_was_text)
//     }
// }
