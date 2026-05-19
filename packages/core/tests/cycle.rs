use dioxus::prelude::*;
use dioxus_renderer_oracle::Sequence;

/// As we clean up old templates, the ID for the node should cycle
#[test]
fn cycling_elements() {
    Sequence::new()
        .render(rsx! { div { "wasd" } })
        .render(rsx! { div { "abcd" } })
        .render(rsx! { div { "wasd" } })
        .render(rsx! { div { "abcd" } })
        .assert_edit_summary(1, |s| {
            assert_eq!(s.loads, 1);
            assert_eq!(s.replaces, 1);
        })
        .assert_edit_summary(2, |s| {
            assert_eq!(s.loads, 1);
            assert_eq!(s.replaces, 1);
        })
        .assert_edit_summary(3, |s| {
            assert_eq!(s.loads, 1);
            assert_eq!(s.replaces, 1);
        })
        .run();
}
