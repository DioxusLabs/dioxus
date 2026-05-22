//! Sledgehammer bindings for the hydration walk script.
//!
//! The Rust side emits *declarative* ops from a pure-VDOM walk; the JS side
//! walks the actual DOM as it consumes ops, doing tag matching and
//! transparent-wrapper tolerance natively. `web-sys` does not appear in
//! `plan.rs` at all — there is no Rust-side DOM read during hydration
//! beyond the one root-filter pass that selects which roots to enter.
//!
//! State shared with the live mutation `BaseInterpreter` lives on `this.base`
//! (set by `installHydrationState`): `nodes`, `createListener`,
//! `createVirtualAnchor`, etc. Walker-local state on `this`: `cursor`,
//! `frames`, `frameWrap`, `lastMapped`, `lastMappedId`, `chainTail`,
//! `currentRootParent`, `hyUnder`.
//!
//! Tag names are interned via the `el` LRU cache (1-byte slot index after
//! the first occurrence), the same cache the mutation interpreter uses for
//! `create_element`. Event names are interned via `evt`.

#[cfg(feature = "webonly")]
#[sledgehammer_bindgen::bindgen(module)]
mod hydration_js {
    // Empty BASE — the hydration class doesn't extend BaseInterpreter, so it
    // doesn't need core.js prepended. State sharing is via `this.base`, set
    // by `installHydrationState` (see `js/hydration_helpers.js`).
    const BASE: &str = "./src/js/hydration_base.js";

    pub struct HydrationChannel;

    /// `EnterRoot(idx)` — `cursor = under[idx]`; clear frames and chain tail.
    /// `currentRootParent` anchors root-level virtual placeholders after the
    /// cursor advances past the last real root.
    fn hy_enter_root(idx: u32) {
        "{this.cursor = this.hyUnder[$idx$] ?? null; this.currentRootParent = (this.cursor && this.cursor.parentNode) || this.base.root; this.frames.length = 0; this.frameWrap.length = 0; this.chainTail = null;}"
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
            this.chainTail = null;
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
        "{this.frames.push(this.cursor); this.frameWrap.push(0); this.cursor = this.cursor ? this.cursor.firstChild : null; this.chainTail = null;}"
    }

    /// `EndChildren` — drain wrapper frames above the top user frame, then
    /// pop the user frame; cursor returns to that parent.
    fn hy_end_children() {
        "{while (this.frameWrap.length > 0 && this.frameWrap[this.frameWrap.length-1] === 1) { this.frames.pop(); this.frameWrap.pop(); } this.cursor = this.frames.pop() ?? null; this.frameWrap.pop(); this.chainTail = null;}"
    }

    /// `Advance(n)` — step `n` `nextSibling`s.
    fn hy_advance(n: u32) {
        "{const n = $n$; for (let k=0; k<n && this.cursor; k++) this.cursor = this.cursor.nextSibling; this.chainTail = null;}"
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
            this.chainTail = null;
        }"#
    }

    /// `SynthText(id)` — virtual sentinel inserted *before* cursor. Chained
    /// via `chainTail` so consecutive synth ops form a linked list resolved
    /// at materialization time.
    fn hy_synth_text(id: u32) {
        r#"{
            const id = $id$;
            const parent = (this.cursor && this.cursor.parentNode) || this.frames[this.frames.length-1] || this.currentRootParent || null;
            const sentinel = this.base.createVirtualAnchor(parent, this.cursor);
            if (this.chainTail) this.chainTail.before = sentinel;
            this.chainTail = sentinel;
            this.base.nodes[id] = sentinel;
        }"#
    }

    /// `SynthTextAfter(id)` — trailing virtual sentinel inserted *after*
    /// cursor. Cursor doesn't advance — virtual sentinels have no DOM
    /// position — but the chain tail keeps source order intact.
    fn hy_synth_text_after(id: u32) {
        r#"{
            const id = $id$;
            const parent = (this.cursor && this.cursor.parentNode) || this.frames[this.frames.length-1] || this.currentRootParent || null;
            const before = this.cursor ? this.cursor.nextSibling : null;
            const sentinel = this.base.createVirtualAnchor(parent, before);
            if (this.chainTail) this.chainTail.before = sentinel;
            this.chainTail = sentinel;
            this.base.nodes[id] = sentinel;
        }"#
    }
}

#[cfg(feature = "webonly")]
#[wasm_bindgen::prelude::wasm_bindgen(module = "/src/js/hydration_helpers.js")]
extern "C" {
    /// One-shot setup before flushing the queued hydration ops. Binds the
    /// HydrationChannel JS instance to the live `BaseInterpreter` (for shared
    /// `nodes`/listener state), installs `hyUnder`, and resets the per-pass
    /// walker state. Must be called before the first sledgehammer flush.
    #[wasm_bindgen(js_name = "installHydrationState")]
    pub fn install_hydration_state(
        channel: &RawHydrationChannel,
        base: &crate::unified_bindings::BaseInterpreter,
        under: Vec<web_sys::Node>,
    );

    /// Push a hydration-only virtual root onto the live mutation stack for an
    /// empty streaming-suspense chunk. The returned sentinel is later claimed
    /// by the resolved scope's first dynamic-root ElementId.
    #[wasm_bindgen(js_name = "pushHydrationVirtualRoot")]
    pub fn push_hydration_virtual_root(
        base: &crate::unified_bindings::BaseInterpreter,
    ) -> wasm_bindgen::JsValue;

    /// Bind a previously pushed hydration virtual root to an ElementId after
    /// `replace_with` has populated its live DOM position.
    #[wasm_bindgen(js_name = "claimHydrationVirtualRoot")]
    pub fn claim_hydration_virtual_root(
        base: &crate::unified_bindings::BaseInterpreter,
        id: u32,
        sentinel: &wasm_bindgen::JsValue,
    );
}
