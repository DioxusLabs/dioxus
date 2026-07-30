//! Exercise `remove_attribute_or_write_fallback` (attributes.rs:240-292)
//! specifically the branch where the disappearing dynamic attribute was
//! shadowing a static template attribute at the same `(name, namespace)`
//! key. After the dynamic disappears, the diff must restore the static
//! value.
//!
//! The fuzz mutator can reach this scenario via its alias-then-remove
//! primitive, but only stochastically. This test pins it down so the
//! coverage of lines 248-292 doesn't depend on fuzz luck.

use dioxus::prelude::*;
use dioxus_core::{Attribute, ScopeId, generation};
use dioxus_renderer_oracle::RendererOracle;

#[test]
fn static_attribute_resurfaces_when_dynamic_disappears() {
    fn app() -> Element {
        // The template carries a *static* `class="from-template"` attribute.
        // On the first generation we layer a *dynamic* `class="overlay"` on
        // top of it via `..attrs`; on the next generation the dynamic
        // attribute disappears, which must restore the static value.
        let attrs: Vec<Attribute> = if generation() == 0 {
            vec![Attribute::new("class", "overlay", None, false)]
        } else {
            Vec::new()
        };

        rsx! {
            div {
                class: "from-template",
                ..attrs,
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    dom.mark_dirty(ScopeId::APP);
    let summary = oracle.render(&mut dom);

    // The dynamic attribute disappears, so the diff must call
    // `remove_attribute_or_write_fallback`, find the static template attr,
    // and emit a `set_attribute` restoring its value. Anything ≥ 1 means
    // the fallback Some(value) branch fired.
    assert!(
        summary.set_attrs >= 1,
        "expected static template attribute to be restored, got summary={summary:?}",
    );
}

#[test]
fn nested_static_attribute_resurfaces_when_dynamic_disappears() {
    // Same scenario as above but on a deeper element path, so
    // `template_node_at_path` recurses through `element_child(...)`
    // (attributes.rs:291) before resolving the owning element.
    fn app() -> Element {
        let attrs: Vec<Attribute> = if generation() == 0 {
            vec![Attribute::new("id", "overlay", None, false)]
        } else {
            Vec::new()
        };

        rsx! {
            section {
                div {
                    span {
                        id: "deep-static",
                        ..attrs,
                    }
                }
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    dom.mark_dirty(ScopeId::APP);
    let summary = oracle.render(&mut dom);

    assert!(
        summary.set_attrs >= 1,
        "expected deep static attribute to be restored, got summary={summary:?}",
    );
}
