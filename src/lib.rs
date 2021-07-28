use anyhow::Result;
use dioxus::core::*;
use tui::backend::CrosstermBackend;
use tui::layout::{Layout, Rect};
use tui::style::{Modifier, Style as TuiStyle};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Paragraph};
use tui_template::tuiapp::CrosstermFrame;
struct RinkDom {
    vdom: VirtualDom,
}

impl RinkDom {
    pub fn new(app: FC<()>) -> Self {
        Self {
            vdom: VirtualDom::new(app),
        }
    }

    fn render_vnode<'a>(
        &self,
        f: &mut CrosstermFrame,
        node: &'a VNode<'a>,
        state: &mut RenderState<'a>,
    ) -> Rect {
        match &node.kind {
            VNodeKind::Fragment(_) => todo!(),
            VNodeKind::Component(_) => todo!(),
            VNodeKind::Suspended { node } => todo!(),

            VNodeKind::Text(te) => {
                let span = Span::styled(te.text, TuiStyle::default());

                let mut m = Modifier::empty();

                for style in &state.current_styles {
                    match style {
                        Styles::Bold => m = m | Modifier::BOLD,
                        Styles::Italic => m = m | Modifier::ITALIC,
                        Styles::Strikethrough => m = m | Modifier::CROSSED_OUT,
                        Styles::Emphasis => m = m | Modifier::ITALIC,
                        Styles::Underline => m = m | Modifier::UNDERLINED,
                    }
                }

                let style = TuiStyle::default().add_modifier(m);
                let span = span.styled_graphemes(style);
                let cur_block = state.block_stack.last_mut().unwrap();

                // Paragraph

                // f.render_widget(span);
            }
            VNodeKind::Element(el) => {
                //
                let mut new_layout = false;

                // all of our supported styles

                match el.tag_name {
                    // obviously semantically not really correct
                    "div" => {
                        state.layouts.push(Layout::default());
                        new_layout = true;
                    }

                    "title" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        let mut block = state.block_stack.pop().unwrap();
                        let children = el.children;

                        if let (1, Some(VNodeKind::Text(te))) =
                            (children.len(), children.get(0).map(|f| &f.kind))
                        {
                            block = block.title(vec![Span::from(te.text)]);
                        }

                        state.block_stack.push(block);
                    }

                    "span" | "header" => {}

                    "footer" => {}

                    "p" => {
                        state.layouts.push(Layout::default());
                        new_layout = true;
                    }

                    // elements that style for whatever reason
                    "em" => state.current_styles.push(Styles::Emphasis),
                    "i" => state.current_styles.push(Styles::Italic),
                    "b" => state.current_styles.push(Styles::Bold),
                    "u" => state.current_styles.push(Styles::Underline),
                    "strike" => state.current_styles.push(Styles::Strikethrough),

                    "li" => {}
                    "ul" => {}
                    "ol" => {}
                    "code" => {}
                    "hr" => {}

                    // Tables
                    "table" => {}
                    "tr" => {}
                    "th" => {}
                    "td" => {}

                    // Inputs
                    "input" => {}
                    "label" => {}

                    _ => {}
                }

                let cur_layout = state.layouts.last_mut().unwrap();
                let cur_block = state.block_stack.last_mut().unwrap();
                let mut cur_style = TuiStyle::default();

                for attr in el.attributes {
                    if attr.namespace == Some("style") {
                        match attr.name {
                            "width" => {}
                            "height" => {}

                            "background" => {
                                //
                                // cur_style.bg
                                // cur_block.style()
                            }
                            "background-color" => {}

                            "border" => {}
                            "border-bottom" => {}
                            "border-bottom-color" => {}
                            "border-bottom-style" => {}
                            "border-bottom-width" => {}
                            "border-color" => {}
                            "border-left" => {}
                            "border-left-color" => {}
                            "border-left-style" => {}
                            "border-left-width" => {}
                            "border-right" => {}
                            "border-right-color" => {}
                            "border-right-style" => {}
                            "border-right-width" => {}
                            "border-style" => {}
                            "border-top" => {}
                            "border-top-color" => {}
                            "border-top-style" => {}
                            "border-top-width" => {}
                            "border-width" => {}

                            "clear" => {}
                            "clip" => {}
                            "color" => {}
                            "cursor" => {}
                            "display" => {}
                            "filter" => {}
                            "float" => {}
                            "font" => {}
                            "font-family" => {}
                            "font-size" => {}
                            "font-variant" => {}
                            "font-weight" => {}

                            "left" => {}
                            "letter-spacing" => {}
                            "line-height" => {}
                            "list-style" => {}
                            "list-style-image" => {}
                            "list-style-position" => {}
                            "list-style-type" => {}
                            "margin" => {}
                            "margin-bottom" => {}
                            "margin-left" => {}
                            "margin-right" => {}
                            "margin-top" => {}
                            "overflow" => {}
                            "padding" => {}
                            "padding-bottom" => {}
                            "padding-left" => {}
                            "padding-right" => {}
                            "padding-top" => {}
                            "position" => {}
                            "stroke-dasharray" => {}
                            "stroke-dashoffset" => {}
                            "text-align" => {}
                            "text-decoration" => {}
                            "text-indent" => {}
                            "text-transform" => {}
                            "top" => {}
                            "vertical-align" => {}
                            "visibility" => {}
                            "z-index" => {}

                            "page-break-after"
                            | "page-break-before"
                            | "background-position"
                            | "background-attachment"
                            | "background-image"
                            | "background-repeat"
                            | _ => {}
                        }
                    }
                }

                for child in el.children {}
            }
        }
        Rect::new(0, 0, 0, 0)
    }

    fn render_text(&self, f: &mut CrosstermFrame, node: &VNode) {}

    fn render_fragment(&self, f: &mut CrosstermFrame) {}
}

impl<'a> tui_template::tuiapp::TuiApp for RinkDom {
    fn render(&mut self, frame: &mut CrosstermFrame) {
        let base_scope = self.vdom.base_scope();
        let root = base_scope.root();

        let mut render_state = RenderState::new();
        self.render_vnode(frame, root, &mut render_state);
    }

    fn event_handler(&self, action: tui_template::crossterm::event::Event) -> Result<()> {
        todo!()
    }

    fn handle_key(&mut self, key: tui_template::crossterm::event::KeyEvent) {
        todo!()
    }

    fn tick(&mut self) {
        todo!()
    }

    fn should_quit(&self) -> bool {
        todo!()
    }
}
struct RenderState<'a> {
    block_stack: Vec<Block<'a>>,

    layouts: Vec<Layout>,

    /// All the current styles applied through the "style" tag
    current_styles: Vec<Styles>,
}

// I don't think we can do any of these?
enum Styles {
    Bold,
    Italic,
    Strikethrough,
    Emphasis,
    Underline,
}

impl<'a> RenderState<'a> {
    fn new() -> Self {
        let block_stack = Vec::new();
        Self {
            block_stack,
            current_styles: Vec::new(),
            layouts: Vec::new(),
        }
    }
}
