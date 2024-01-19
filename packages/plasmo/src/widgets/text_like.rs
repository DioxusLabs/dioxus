use std::{collections::HashMap, io::stdout};

use crossterm::{cursor::MoveTo, execute};
use dioxus_html::{
    input_data::keyboard_types::Key, prelude::*, HasKeyboardData, SerializedKeyboardData,
    SerializedMouseData,
};
use dioxus_native_core::{
    custom_element::CustomElement,
    node::OwnedAttributeDiscription,
    node_ref::AttributeMask,
    prelude::{ElementNode, NodeType},
    real_dom::{ElementNodeMut, NodeImmutable, NodeMut, NodeTypeMut, RealDom},
    utils::cursor::{Cursor, Pos},
    NodeId,
};
use shipyard::UniqueView;
use taffy::geometry::Point;

use crate::hooks::FormData;
use crate::{query::get_layout, Event, EventData, Query};

use super::{RinkWidget, WidgetContext};

pub(crate) trait TextLikeController {
    fn display_text(&self, text: &str) -> String {
        text.to_string()
    }
}

#[derive(Debug, Default)]
pub(crate) struct EmptyController;

impl TextLikeController for EmptyController {}

#[derive(Debug, Default)]
pub(crate) struct TextLike<C: TextLikeController = EmptyController> {
    text: String,
    div_wrapper: NodeId,
    pre_cursor_text: NodeId,
    highlighted_text: NodeId,
    post_cursor_text: NodeId,
    cursor: Cursor,
    dragging: bool,
    border: bool,
    max_len: Option<usize>,
    controller: C,
}

impl<C: TextLikeController> TextLike<C> {
    fn width(el: &ElementNodeMut) -> String {
        if let Some(value) = el
            .get_attribute(&OwnedAttributeDiscription {
                name: "width".to_string(),
                namespace: Some("style".to_string()),
            })
            .and_then(|value| value.as_text())
            .map(|value| value.to_string())
        {
            value
        } else {
            "1px".to_string()
        }
    }

    fn height(el: &ElementNodeMut) -> String {
        if let Some(value) = el
            .get_attribute(&OwnedAttributeDiscription {
                name: "height".to_string(),
                namespace: Some("style".to_string()),
            })
            .and_then(|value| value.as_text())
            .map(|value| value.to_string())
        {
            value
        } else {
            "1px".to_string()
        }
    }

    fn update_max_width_attr(&mut self, el: &ElementNodeMut) {
        if let Some(value) = el
            .get_attribute(&OwnedAttributeDiscription {
                name: "maxlength".to_string(),
                namespace: None,
            })
            .and_then(|value| value.as_text())
            .map(|value| value.to_string())
        {
            if let Ok(max_len) = value.parse::<usize>() {
                self.max_len = Some(max_len);
            }
        }
    }

    fn update_size_attr(&mut self, el: &mut ElementNodeMut) {
        let width = Self::width(el);
        let height = Self::height(el);
        let single_char = width
            .strip_prefix("px")
            .and_then(|n| n.parse::<u32>().ok().filter(|num| *num > 3))
            .is_some()
            || height
                .strip_prefix("px")
                .and_then(|n| n.parse::<u32>().ok().filter(|num| *num > 3))
                .is_some();
        self.border = !single_char;
        let border_style = if self.border { "solid" } else { "none" };
        el.set_attribute(
            OwnedAttributeDiscription {
                name: "border-style".to_string(),
                namespace: Some("style".to_string()),
            },
            border_style.to_string(),
        );
    }

    fn update_value_attr(&mut self, el: &ElementNodeMut) {
        if let Some(value) = el
            .get_attribute(&OwnedAttributeDiscription {
                name: "value".to_string(),
                namespace: None,
            })
            .and_then(|value| value.as_text())
            .map(|value| value.to_string())
        {
            self.text = value;
        }
    }

    pub(crate) fn set_text(&mut self, text: String, rdom: &mut RealDom, id: NodeId) {
        self.text = text;
        self.write_value(rdom, id);
    }

    pub(crate) fn text(&self) -> &str {
        self.text.as_str()
    }

    fn write_value(&self, rdom: &mut RealDom, id: NodeId) {
        let start_highlight = self.cursor.first().idx(self.text.as_str());
        let end_highlight = self.cursor.last().idx(self.text.as_str());
        let (text_before_first_cursor, text_after_first_cursor) =
            self.text.split_at(start_highlight);
        let (text_highlighted, text_after_second_cursor) =
            text_after_first_cursor.split_at(end_highlight - start_highlight);

        if let Some(mut text) = rdom.get_mut(self.pre_cursor_text) {
            let node_type = text.node_type_mut();
            let NodeTypeMut::Text(mut text) = node_type else {
                panic!("input must be an element")
            };
            *text.text_mut() = self.controller.display_text(text_before_first_cursor);
        }

        if let Some(mut text) = rdom.get_mut(self.highlighted_text) {
            let node_type = text.node_type_mut();
            let NodeTypeMut::Text(mut text) = node_type else {
                panic!("input must be an element")
            };
            *text.text_mut() = self.controller.display_text(text_highlighted);
        }

        if let Some(mut text) = rdom.get_mut(self.post_cursor_text) {
            let node_type = text.node_type_mut();
            let NodeTypeMut::Text(mut text) = node_type else {
                panic!("input must be an element")
            };
            *text.text_mut() = self.controller.display_text(text_after_second_cursor);
        }

        // send the event
        {
            let world = rdom.raw_world_mut();
            let data: FormData = FormData {
                value: self.text.clone(),
                values: HashMap::new(),
                files: None,
            };
            let ctx: UniqueView<WidgetContext> = world.borrow().expect("expected widget context");

            ctx.send(Event {
                id,
                name: "input",
                data: EventData::Form(data),
                bubbles: true,
            });
        }
    }

    fn handle_keydown(&mut self, mut root: NodeMut, data: &SerializedKeyboardData) {
        let key = data.key();
        let modifiers = data.modifiers();
        let code = data.code();

        if key == Key::Enter {
            return;
        }
        self.cursor.handle_input(
            &code,
            &key,
            &modifiers,
            &mut self.text,
            self.max_len.unwrap_or(1000),
        );

        let id = root.id();

        let rdom = root.real_dom_mut();
        self.write_value(rdom, id);
        let world = rdom.raw_world_mut();

        // move cursor to new position
        let taffy = {
            let query: UniqueView<Query> = world.borrow().unwrap();
            query.stretch.clone()
        };

        let taffy = taffy.lock().unwrap();

        let layout = get_layout(rdom.get(self.div_wrapper).unwrap(), &taffy).unwrap();
        let Point { x, y } = layout.location;

        let Pos { col, row } = self.cursor.start;
        let (x, y) = (col as u16 + x as u16, row as u16 + y as u16);
        if let Ok(pos) = crossterm::cursor::position() {
            if pos != (x, y) {
                execute!(stdout(), MoveTo(x, y)).unwrap();
            }
        } else {
            execute!(stdout(), MoveTo(x, y)).unwrap();
        }
    }

    fn handle_mousemove(&mut self, mut root: NodeMut, data: &SerializedMouseData) {
        if self.dragging {
            let id = root.id();
            let offset = data.element_coordinates();
            let mut new = Pos::new(offset.x as usize, offset.y as usize);

            // textboxs are only one line tall
            new.row = 0;

            if new != self.cursor.start {
                self.cursor.end = Some(new);
            }
            let rdom = root.real_dom_mut();
            self.write_value(rdom, id);
        }
    }

    fn handle_mousedown(&mut self, mut root: NodeMut, data: &SerializedMouseData) {
        let offset = data.element_coordinates();
        let mut new = Pos::new(offset.x as usize, offset.y as usize);

        // textboxs are only one line tall
        new.row = 0;

        new.realize_col(self.text.as_str());
        self.cursor = Cursor::from_start(new);
        self.dragging = true;

        let id = root.id();

        // move cursor to new position
        let rdom = root.real_dom_mut();
        let world = rdom.raw_world_mut();
        let taffy = {
            let query: UniqueView<Query> = world.borrow().unwrap();
            query.stretch.clone()
        };

        let taffy = taffy.lock().unwrap();

        let layout = get_layout(rdom.get(self.div_wrapper).unwrap(), &taffy).unwrap();
        let Point { x, y } = layout.location;

        let Pos { col, row } = self.cursor.start;
        let (x, y) = (col as u16 + x as u16, row as u16 + y as u16);
        if let Ok(pos) = crossterm::cursor::position() {
            if pos != (x, y) {
                execute!(stdout(), MoveTo(x, y)).unwrap();
            }
        } else {
            execute!(stdout(), MoveTo(x, y)).unwrap();
        }

        self.write_value(rdom, id)
    }
}

impl<C: TextLikeController + Send + Sync + Default + 'static> CustomElement for TextLike<C> {
    const NAME: &'static str = "input";

    fn roots(&self) -> Vec<NodeId> {
        vec![self.div_wrapper]
    }

    fn create(mut root: dioxus_native_core::real_dom::NodeMut) -> Self {
        let node_type = root.node_type();
        let NodeType::Element(el) = &*node_type else {
            panic!("input must be an element")
        };

        let value = el
            .attributes
            .get(&OwnedAttributeDiscription {
                name: "value".to_string(),
                namespace: None,
            })
            .and_then(|value| value.as_text())
            .map(|value| value.to_string());

        drop(node_type);

        let rdom = root.real_dom_mut();

        let pre_text = rdom.create_node(String::new());
        let pre_text_id = pre_text.id();
        let highlighted_text = rdom.create_node(String::new());
        let highlighted_text_id = highlighted_text.id();
        let mut highlighted_text_span = rdom.create_node(NodeType::Element(ElementNode {
            tag: "span".to_string(),
            attributes: [(
                OwnedAttributeDiscription {
                    name: "background-color".to_string(),
                    namespace: Some("style".to_string()),
                },
                "rgba(255, 255, 255, 50%)".to_string().into(),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        }));
        highlighted_text_span.add_child(highlighted_text_id);
        let highlighted_text_span_id = highlighted_text_span.id();
        let post_text = rdom.create_node(value.clone().unwrap_or_default());
        let post_text_id = post_text.id();
        let mut div_wrapper = rdom.create_node(NodeType::Element(ElementNode {
            tag: "div".to_string(),
            attributes: [(
                OwnedAttributeDiscription {
                    name: "display".to_string(),
                    namespace: Some("style".to_string()),
                },
                "flex".to_string().into(),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        }));
        let div_wrapper_id = div_wrapper.id();
        div_wrapper.add_child(pre_text_id);
        div_wrapper.add_child(highlighted_text_span_id);
        div_wrapper.add_child(post_text_id);

        div_wrapper.add_event_listener("mousemove");
        div_wrapper.add_event_listener("mousedown");
        div_wrapper.add_event_listener("mouseup");
        div_wrapper.add_event_listener("mouseleave");
        div_wrapper.add_event_listener("mouseenter");
        root.add_event_listener("keydown");
        root.add_event_listener("focusout");

        Self {
            pre_cursor_text: pre_text_id,
            highlighted_text: highlighted_text_id,
            post_cursor_text: post_text_id,
            div_wrapper: div_wrapper_id,
            cursor: Cursor::default(),
            text: value.unwrap_or_default(),
            ..Default::default()
        }
    }

    fn attributes_changed(
        &mut self,
        mut root: dioxus_native_core::real_dom::NodeMut,
        attributes: &dioxus_native_core::node_ref::AttributeMask,
    ) {
        match attributes {
            AttributeMask::All => {
                {
                    let node_type = root.node_type_mut();
                    let NodeTypeMut::Element(mut el) = node_type else {
                        panic!("input must be an element")
                    };
                    self.update_value_attr(&el);
                    self.update_size_attr(&mut el);
                    self.update_max_width_attr(&el);
                }
                let id = root.id();
                self.write_value(root.real_dom_mut(), id);
            }
            AttributeMask::Some(attrs) => {
                {
                    let node_type = root.node_type_mut();
                    let NodeTypeMut::Element(mut el) = node_type else {
                        panic!("input must be an element")
                    };
                    if attrs.contains("width") || attrs.contains("height") {
                        self.update_size_attr(&mut el);
                    }
                    if attrs.contains("maxlength") {
                        self.update_max_width_attr(&el);
                    }
                    if attrs.contains("value") {
                        self.update_value_attr(&el);
                    }
                }
                if attrs.contains("value") {
                    let id = root.id();
                    self.write_value(root.real_dom_mut(), id);
                }
            }
        }
    }
}

impl<C: TextLikeController + Send + Sync + Default + 'static> RinkWidget for TextLike<C> {
    fn handle_event(&mut self, event: &crate::Event, node: NodeMut) {
        match event.name {
            "keydown" => {
                if let EventData::Keyboard(data) = &event.data {
                    self.handle_keydown(node, data);
                }
            }

            "mousemove" => {
                if let EventData::Mouse(data) = &event.data {
                    self.handle_mousemove(node, data);
                }
            }

            "mousedown" => {
                if let EventData::Mouse(data) = &event.data {
                    self.handle_mousedown(node, data);
                }
            }

            "mouseup" => {
                self.dragging = false;
            }

            "mouseleave" => {
                self.dragging = false;
            }

            "mouseenter" => {
                self.dragging = false;
            }

            "focusout" => {
                execute!(stdout(), MoveTo(0, 1000)).unwrap();
            }

            _ => {}
        }
    }
}
