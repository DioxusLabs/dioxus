//! Sledgehammer bindings for the hydration walk script.
//!
//! The Rust side emits *declarative* ops from a pure-VDOM walk; the JS side
//! walks the actual DOM as it consumes ops, doing tag matching and
//! transparent-wrapper tolerance natively. `web-sys` does not appear in
//! `plan.rs` at all — there is no Rust-side DOM read during hydration
//! beyond the one root-filter pass that selects which roots to enter.
//!
//! State shared with the live mutation `BaseInterpreter` lives on `this.base`
//! (set by `bind_hydration_channel`): `nodes`, `createListener`, etc.
//! Walker-local state on `this`: `cursor`, `frames`, `frameWrap`,
//! `lastMapped`, `lastMappedId`, `currentRootParent`, `hyUnder`.
//!
//! Tag names are interned via the `el` LRU cache (1-byte slot index after
//! the first occurrence), the same cache the mutation interpreter uses for
//! `create_element`. Event names are interned via `evt`.

#[cfg(feature = "webonly")]
#[sledgehammer_bindgen::bindgen(module)]
mod hydration_js {
    // Empty BASE — the hydration class doesn't extend BaseInterpreter, so it
    // doesn't need core.js prepended. State sharing is via `this.base`, set
    // by `bind_hydration_channel` before the first flush.
    const BASE: &str = "./src/js/hydration_base.js";

    pub struct HydrationChannel;

    /// `EnterRoot(idx)` — `cursor = under[idx]`; clear frames.
    /// `currentRootParent` is the insertion parent for synthesized empty text
    /// nodes once the cursor advances past the last real root.
    fn hy_enter_root(idx: u32) {
        "{this.cursor = this.hyUnder[$idx$] ?? null; this.currentRootParent = (this.cursor && this.cursor.parentNode) || this.base.root; this.frames = []; this.frameWrap = [];}"
    }

    /// `MapElement(tag, id)` — verify cursor element matches `tag` (with
    /// transparent-wrapper auto-descent for attr-less mismatched elements);
    /// map to `id` if `id != 0`; record `lastMapped`/`lastMappedId` for any
    /// subsequent `AttachListener`.
    ///
    /// Auto-descent handles parser-inserted wrappers (e.g. `<tbody>`). Each
    /// auto-descent pushes the wrapper onto `frames` with `frameWrap[i] = 1`,
    /// so `EndChildren` later pops the wrapper alongside its containing user
    /// frame.
    fn hy_map_element(tag: &str<u8, el>, id: u32) {
        // Locals cache `$tag$` and `$id$` — sledgehammer rewrites every
        // `$arg$` token as a fresh cache/buffer read, so multiple uses in one
        // body would each pop new bytes off the op stream and corrupt
        // subsequent ops.
        r#"{
            const tag = $tag$;
            const id = $id$;
            while (this.cursor && this.cursor.nodeType === 1 && this.cursor.localName !== tag && !this.cursor.hasAttributes()) {
                this.frames.push(this.cursor);
                this.frameWrap.push(1);
                this.cursor = this.cursor.firstChild;
            }
            if (!this.cursor || this.cursor.nodeType !== 1 || this.cursor.localName !== tag) {
                throw new Error("dioxus hydration: expected <" + tag + "> but cursor is " + (this.cursor ? this.cursor.nodeName : "null"));
            }
            if (id !== 0) {
                this.base.nodes[id] = this.cursor;
            }
            this.lastMapped = this.cursor;
            this.lastMappedId = id;
        }"#
    }

    /// `AttachListener(name, bubbles)` — bind a delegated event handler to
    /// the most recently mapped element.
    fn hy_attach_listener(event_name: &str<u8, evt>, bubbles: u8) {
        r#"{
            const el = this.lastMapped;
            if (!(el instanceof Element)) {
                throw new Error("dioxus hydration: listener target is not an Element");
            }
            el.listening = (el.listening || 0) + 1;
            el.setAttribute("data-dioxus-id", this.lastMappedId.toString());
            this.base.createListener($event_name$, el, $bubbles$ === 1);
        }"#
    }

    /// `BeginChildren` — push cursor as a user frame; descend to firstChild.
    fn hy_begin_children() {
        "{this.frames.push(this.cursor); this.frameWrap.push(0); this.cursor = this.cursor ? this.cursor.firstChild : null;}"
    }

    /// `EndChildren` — drain wrapper frames above the top user frame, then
    /// pop the user frame; cursor returns to that parent.
    fn hy_end_children() {
        "{while (this.frameWrap.length > 0 && this.frameWrap[this.frameWrap.length-1] === 1) { this.frames.pop(); this.frameWrap.pop(); } this.cursor = this.frames.pop() ?? null; this.frameWrap.pop();}"
    }

    /// `Advance(n)` — step `n` `nextSibling`s.
    fn hy_advance(n: u32) {
        "{const n = $n$; for (let k=0; k<n && this.cursor; k++) this.cursor = this.cursor.nextSibling;}"
    }

    /// `TextContrib(utf16_len, id, split_after)` — Map cursor to `id` if
    /// `id != 0`; if `split_after == 1`, call `cursor.splitText(utf16_len)`
    /// and advance to the new node.
    ///
    /// Used for every contribution to a browser-merged text run except
    /// trailing empties (those use `SynthTextAfter`) and embedded
    /// placeholders (those use `Placeholder`).
    fn hy_text_contrib(utf16_len: u32, id: u32, split_after: u8) {
        r#"{
            const len = $utf16_len$;
            const id = $id$;
            const splitAfter = $split_after$;
            if (!this.cursor || this.cursor.nodeType !== 3) {
                throw new Error("dioxus hydration: expected text node, got " + (this.cursor ? this.cursor.nodeName : "null"));
            }
            if (id !== 0) {
                this.base.nodes[id] = this.cursor;
            }
            if (splitAfter === 1) {
                if (len > this.cursor.length) {
                    throw new Error("dioxus hydration: splitText len=" + len + " > cursor.length=" + this.cursor.length);
                }
                this.cursor = this.cursor.splitText(len);
            }
        }"#
    }

    /// `SynthText(id)` — empty text nodes don't survive HTML serialization, so
    /// the server never emits one. Create a real empty text node, insert it
    /// *before* the cursor, and map `id` to it. The cursor stays put (the new
    /// node sits behind it), so consecutive `SynthText` ops accumulate in
    /// source order and the run consumes no extra DOM sibling.
    fn hy_synth_text(id: u32) {
        r#"{
            const id = $id$;
            const parent = (this.cursor && this.cursor.parentNode) || this.frames[this.frames.length-1] || this.currentRootParent || null;
            const node = document.createTextNode("");
            parent.insertBefore(node, this.cursor || null);
            this.base.nodes[id] = node;
        }"#
    }

    /// `SynthTextAfter(id)` — trailing empty text node inserted *after* the
    /// cursor. Create a real empty text node, insert it after the cursor, then
    /// advance the cursor onto it. Advancing keeps consecutive trailing synths
    /// in source order and keeps the run's consumed-sibling count at one (the
    /// caller still advances past exactly one node to reach the next sibling).
    fn hy_synth_text_after(id: u32) {
        r#"{
            const id = $id$;
            const parent = (this.cursor && this.cursor.parentNode) || this.frames[this.frames.length-1] || this.currentRootParent || null;
            const before = this.cursor ? this.cursor.nextSibling : null;
            const node = document.createTextNode("");
            parent.insertBefore(node, before);
            this.base.nodes[id] = node;
            this.cursor = node;
        }"#
    }
}

/// Wire a fresh [`HydrationChannel`] to the live mutation `BaseInterpreter`
/// (shared `nodes` / listener state) and the SSR root nodes that `EnterRoot(i)`
/// indexes into. Must run before the first flush; the channel's per-pass walker
/// state is (re)initialized by `EnterRoot` itself.
#[cfg(feature = "webonly")]
pub fn bind_hydration_channel(
    channel: &RawHydrationChannel,
    base: &crate::unified_bindings::BaseInterpreter,
    under: Vec<web_sys::Node>,
) {
    use wasm_bindgen::JsValue;
    let channel: &JsValue = channel.as_ref();
    let under: js_sys::Array = under.into_iter().map(JsValue::from).collect();
    let _ = js_sys::Reflect::set(channel, &JsValue::from_str("base"), base.as_ref());
    let _ = js_sys::Reflect::set(channel, &JsValue::from_str("hyUnder"), under.as_ref());
}
