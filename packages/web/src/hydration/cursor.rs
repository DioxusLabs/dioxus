//! A DOM cursor that hydrates server-rendered markup by walking the real DOM
//! in lockstep with a pure-VDOM walk.
//!
//! The SSR HTML carries **no** hydration markers. Instead, [`super::walk`]
//! walks the rebuilt VDOM and calls these cursor methods, which navigate the
//! actual DOM via `web-sys` (tag matching, transparent-wrapper tolerance, text
//! splitting) and bind nodes/listeners on the live mutation interpreter. Because
//! the walk runs synchronously in Rust, structural mismatches surface here as a
//! [`RehydrationError::HydrationMismatch`] instead of a thrown JS exception, and
//! richer mismatch diagnostics can be attached at these sites later.
//!
//! State mirrors the previous JS walker exactly: `cursor` is the current DOM
//! node; `frames`/`frame_wrap` is a stack of saved cursors where wrapper frames
//! (parser-inserted elements auto-descended past) are drained before the
//! enclosing user frame; `current_root_parent` is the insertion parent for
//! synthesized empty text nodes once the cursor advances past the last real root.

use dioxus_interpreter_js::unified_bindings::BaseInterpreter;
use wasm_bindgen::{JsCast, JsValue};

use super::RehydrationError;
use super::RehydrationError::HydrationMismatch;

pub(super) struct HydrationCursor {
    /// The DOM node currently under inspection.
    cursor: Option<web_sys::Node>,
    /// Saved cursors for `begin_children`/auto-descent. An entry may be `None`
    /// because the JS walker pushed a null cursor when descending an empty list.
    frames: Vec<Option<web_sys::Node>>,
    /// Parallel to `frames`; `true` marks a transparent-wrapper frame that
    /// `end_children` drains before popping the enclosing user frame.
    frame_wrap: Vec<bool>,
    /// The SSR root nodes that `enter_root` indexes into.
    under: Vec<web_sys::Node>,
    /// The mount element; the fallback parent for `current_root_parent`.
    root: web_sys::Node,
    /// Insertion parent for synthesized empty text nodes.
    current_root_parent: web_sys::Node,
    /// Cached document handle for creating empty text nodes.
    document: web_sys::Document,
    /// The live mutation interpreter (stored as a `JsValue` because
    /// `BaseInterpreter` is not `Clone`); used to bind nodes and attach
    /// listeners. Reconstructed as `&BaseInterpreter` via [`Self::base`].
    base: JsValue,
}

impl HydrationCursor {
    pub(super) fn new(
        base: &BaseInterpreter,
        root: web_sys::Node,
        under: Vec<web_sys::Node>,
    ) -> Self {
        let document = web_sys::window().unwrap().document().unwrap();
        Self {
            cursor: None,
            frames: Vec::new(),
            frame_wrap: Vec::new(),
            under,
            current_root_parent: root.clone(),
            root,
            document,
            base: base.unchecked_ref::<JsValue>().clone(),
        }
    }

    /// The live mutation interpreter, reinterpreted from the stored `JsValue`.
    fn base(&self) -> &BaseInterpreter {
        self.base.unchecked_ref()
    }

    /// `EnterRoot(idx)` — park the cursor on `under[idx]` and clear frames.
    /// `current_root_parent` is the cursor's parent (or the mount root when the
    /// cursor has none / is out of range).
    pub(super) fn enter_root(&mut self, idx: usize) {
        self.cursor = self.under.get(idx).cloned();
        self.current_root_parent = self
            .cursor
            .as_ref()
            .and_then(|c| c.parent_node())
            .unwrap_or_else(|| self.root.clone());
        self.frames.clear();
        self.frame_wrap.clear();
    }

    /// `MapElement(tag, id)` — descend through attr-less wrapper elements whose
    /// tag mismatches (parser-inserted `<tbody>` etc.), then require the cursor
    /// to be an element with the expected `tag`. Binds `id` (when non-zero) and
    /// returns the matched element so the caller can attach listeners.
    pub(super) fn map_element(
        &mut self,
        tag: &str,
        id: u32,
    ) -> Result<web_sys::Element, RehydrationError> {
        // Transparent-wrapper auto-descent.
        loop {
            let Some(node) = self.cursor.clone() else {
                break;
            };
            let Some(el) = node.dyn_ref::<web_sys::Element>() else {
                break;
            };
            if el.local_name() == tag || el.has_attributes() {
                break;
            }
            self.frames.push(Some(node.clone()));
            self.frame_wrap.push(true);
            self.cursor = node.first_child();
        }

        let node = self.cursor.clone().ok_or(HydrationMismatch)?;
        let el = node
            .dyn_ref::<web_sys::Element>()
            .ok_or(HydrationMismatch)?;
        if el.local_name() != tag {
            return Err(HydrationMismatch);
        }
        if id != 0 {
            self.base().set_node(id, &node);
        }
        Ok(el.clone())
    }

    /// `AttachListener(name, bubbles)` — bind a delegated handler to a mapped
    /// element. Mirrors `addTopEventListener`: bump the `listening` count, set
    /// `data-dioxus-id` *before* `createListener` (its non-bubbling path reads
    /// that attribute), then register the listener.
    pub(super) fn attach_listener(
        &self,
        el: &web_sys::Element,
        id: u32,
        name: &str,
        bubbles: bool,
    ) {
        let key = JsValue::from_str("listening");
        let current = js_sys::Reflect::get(el, &key)
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let _ = js_sys::Reflect::set(el, &key, &JsValue::from_f64(current + 1.0));
        let _ = el.set_attribute("data-dioxus-id", &id.to_string());
        self.base().create_listener(name, el, bubbles);
    }

    /// `BeginChildren` — push the cursor as a user frame and descend to its
    /// first child.
    pub(super) fn begin_children(&mut self) {
        let frame = self.cursor.clone();
        self.cursor = frame.as_ref().and_then(|c| c.first_child());
        self.frames.push(frame);
        self.frame_wrap.push(false);
    }

    /// `EndChildren` — drain trailing wrapper frames, then pop the user frame
    /// the cursor descended from.
    pub(super) fn end_children(&mut self) {
        while matches!(self.frame_wrap.last(), Some(true)) {
            self.frames.pop();
            self.frame_wrap.pop();
        }
        self.cursor = self.frames.pop().flatten();
        self.frame_wrap.pop();
    }

    /// `Advance(n)` — step `n` next-siblings, stopping at the end of the list.
    pub(super) fn advance(&mut self, n: u32) {
        for _ in 0..n {
            match self.cursor.take() {
                Some(node) => self.cursor = node.next_sibling(),
                None => break,
            }
        }
    }

    /// `TextContrib(len, id, split_after)` — require the cursor to be a text
    /// node, bind `id` (when non-zero), and optionally split the run at `len`
    /// UTF-16 units, advancing the cursor onto the new trailing text node.
    pub(super) fn text_contrib(
        &mut self,
        len: u32,
        id: u32,
        split_after: bool,
    ) -> Result<(), RehydrationError> {
        let node = self.cursor.clone().ok_or(HydrationMismatch)?;
        let text = node.dyn_ref::<web_sys::Text>().ok_or(HydrationMismatch)?;
        if id != 0 {
            self.base().set_node(id, &node);
        }
        if split_after {
            let new_text = text.split_text(len).map_err(|_| HydrationMismatch)?;
            self.cursor = Some(new_text.unchecked_into());
        }
        Ok(())
    }

    /// `SynthText(id)` — empty text nodes don't survive HTML serialization.
    /// Create one, insert it *before* the cursor (the cursor stays put so
    /// consecutive synths accumulate in source order), and bind `id`.
    pub(super) fn synth_text(&mut self, id: u32) -> Result<(), RehydrationError> {
        let parent = self.synth_parent();
        let node = self.document.create_text_node("");
        parent
            .insert_before(node.as_ref(), self.cursor.as_ref())
            .map_err(|_| HydrationMismatch)?;
        self.base().set_node(id, node.as_ref());
        Ok(())
    }

    /// `SynthTextAfter(id)` — create an empty text node *after* the cursor, bind
    /// `id`, and advance the cursor onto it (keeping the run's consumed-sibling
    /// count at one).
    pub(super) fn synth_text_after(&mut self, id: u32) -> Result<(), RehydrationError> {
        let parent = self.synth_parent();
        let before = self.cursor.as_ref().and_then(|c| c.next_sibling());
        let node = self.document.create_text_node("");
        parent
            .insert_before(node.as_ref(), before.as_ref())
            .map_err(|_| HydrationMismatch)?;
        self.base().set_node(id, node.as_ref());
        self.cursor = Some(node.unchecked_into());
        Ok(())
    }

    /// Insertion parent for a synthesized text node: the cursor's parent, else
    /// the current top frame, else the root parent.
    fn synth_parent(&self) -> web_sys::Node {
        self.cursor
            .as_ref()
            .and_then(|c| c.parent_node())
            .or_else(|| self.frames.last().cloned().flatten())
            .unwrap_or_else(|| self.current_root_parent.clone())
    }
}
