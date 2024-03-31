use dioxus_core::prelude::use_drop;

#[deprecated(note = "Use `use_drop` instead, which has the same functionality.")]
pub fn use_on_unmount<D: FnOnce() + 'static>(destroy: D) {
    use_drop(destroy);
}
